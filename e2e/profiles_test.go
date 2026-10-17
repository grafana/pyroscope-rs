package e2e

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/google/pprof/profile"
	querierv1 "github.com/grafana/pyroscope/api/gen/proto/go/querier/v1"
	"google.golang.org/protobuf/encoding/protojson"
	"google.golang.org/protobuf/proto"
)

const (
	allocationMarker   = "allocate_server_event_marker"
	liveMarker         = "allocate_server_live_marker"
	freedControlMarker = "allocate_server_freed_control_marker"
	retainedSize       = 2 * 1024 * 1024
	smallSize          = 64 * 1024
)

func verifyMimallocProfiles(t *testing.T, ctx context.Context, url, service, artifacts string) {
	t.Helper()
	inputs := make(map[string]*profile.Profile)
	for _, input := range []struct{ name, marker string }{
		{"events-first", allocationMarker},
		{"events-second", allocationMarker},
		{"retained", liveMarker},
		{"freed", freedControlMarker},
	} {
		data, err := os.ReadFile(filepath.Join(artifacts, input.name+"-uploaded.pprof"))
		if err != nil {
			t.Fatal(err)
		}
		p, err := profile.ParseData(data)
		if err != nil {
			t.Fatalf("decode %s input: %v", input.name, err)
		}
		if err := applicationFrames(p); err != nil {
			t.Fatalf("%s input: %v", input.name, err)
		}
		if err := sourceMetadata(p, input.marker, true); err != nil {
			t.Fatalf("%s input: %v", input.name, err)
		}
		inputs[input.name] = p
	}
	// Check the fixture independently before comparing it with server queries.
	for _, input := range []struct {
		name, marker, kind string
		want               int64
	}{
		{"events-first", allocationMarker, "alloc_objects", 1},
		{"events-first", allocationMarker, "alloc_space", retainedSize},
		{"events-second", allocationMarker, "alloc_objects", 1},
		{"events-second", allocationMarker, "alloc_space", smallSize},
		{"retained", liveMarker, "inuse_objects", 1},
		{"retained", liveMarker, "inuse_space", retainedSize},
		{"freed", liveMarker, "inuse_objects", 0},
		{"freed", liveMarker, "inuse_space", 0},
		{"freed", freedControlMarker, "inuse_objects", 1},
		{"freed", freedControlMarker, "inuse_space", smallSize},
	} {
		if got := markerValue(inputs[input.name], input.marker, input.kind); got != input.want {
			t.Fatalf("%s input %s: got %d, want %d", input.name, input.kind, got, input.want)
		}
	}

	client := &http.Client{Timeout: 5 * time.Second}
	now := time.Now().UnixMilli()
	for _, check := range []struct {
		phase, kind, marker string
		want                int64
	}{
		{"events", "alloc_objects", allocationMarker, 2},
		{"events", "alloc_space", allocationMarker, retainedSize + smallSize},
		{"retained", "inuse_objects", liveMarker, 1},
		{"retained", "inuse_space", liveMarker, retainedSize},
	} {
		unit := "bytes"
		if strings.HasSuffix(check.kind, "objects") {
			unit = "count"
		}
		request := &querierv1.SelectMergeStacktracesRequest{
			ProfileTypeID: fmt.Sprintf("memory:%s:%s:space:bytes", check.kind, unit),
			LabelSelector: fmt.Sprintf(`{service_name=%q,phase=%q,backend="mimalloc"}`, service, check.phase),
			Start:         now - 60_000,
			End:           now + 60_000,
			Format:        querierv1.ProfileFormat_PROFILE_FORMAT_PPROF,
		}
		p := waitForProfile(t, ctx, client, url, request, check.marker, check.kind, check.want)
		if err := sourceMetadata(p, check.marker, false); err != nil {
			t.Fatalf("%s/%s query: %v", check.phase, check.kind, err)
		}
		saveProfile(t, artifacts, check.phase+"-"+check.kind+"-queried.pprof", p)

		flameRequest := proto.Clone(request).(*querierv1.SelectMergeStacktracesRequest)
		flameRequest.Format = querierv1.ProfileFormat_PROFILE_FORMAT_FLAMEGRAPH
		var flamegraph querierv1.SelectMergeStacktracesResponse
		if err := query(ctx, client, url, "SelectMergeStacktraces", flameRequest, &flamegraph); err != nil {
			t.Fatal(err)
		}
		saveJSON(t, artifacts, check.phase+"-"+check.kind+"-flamegraph.json", &flamegraph)
		found := false
		for _, name := range flamegraph.GetFlamegraph().GetNames() {
			if strings.Contains(name, check.marker) {
				found = true
				break
			}
		}
		if !found {
			t.Fatalf("application marker missing from %s/%s flamegraph", check.phase, check.kind)
		}
		t.Logf("verified phase=%s kind=%s value=%d", check.phase, check.kind, check.want)

		if strings.HasPrefix(check.kind, "inuse") {
			freedRequest := proto.Clone(request).(*querierv1.SelectMergeStacktracesRequest)
			freedRequest.LabelSelector = fmt.Sprintf(`{service_name=%q,phase="freed",backend="mimalloc"}`, service)
			// An ingested positive control is required before asserting absence.
			wantControl := int64(1)
			if check.kind == "inuse_space" {
				wantControl = smallSize
			}
			freed := waitForProfile(t, ctx, client, url, freedRequest, freedControlMarker, check.kind, wantControl)
			if got := markerValue(freed, liveMarker, check.kind); got != 0 {
				t.Fatalf("freed snapshot still contains %d %s for %s", got, check.kind, liveMarker)
			}
			saveProfile(t, artifacts, "freed-"+check.kind+"-queried.pprof", freed)
		}
	}

	var types querierv1.ProfileTypesResponse
	if err := query(ctx, client, url, "ProfileTypes", &querierv1.ProfileTypesRequest{
		Start: now - 60_000, End: now + 60_000,
	}, &types); err != nil {
		t.Fatal(err)
	}
	saveJSON(t, artifacts, "profile-types.json", &types)
	for _, kind := range []string{"alloc_objects", "alloc_space", "inuse_objects", "inuse_space"} {
		found := false
		for _, typ := range types.GetProfileTypes() {
			if strings.HasPrefix(typ.GetID(), "memory:"+kind+":") {
				found = true
				break
			}
		}
		if !found {
			t.Fatalf("profile type %s not advertised by server", kind)
		}
	}
}

func waitForProfile(t *testing.T, ctx context.Context, client *http.Client, url string,
	request *querierv1.SelectMergeStacktracesRequest, marker, kind string, want int64,
) *profile.Profile {
	t.Helper()
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	ticker := time.NewTicker(200 * time.Millisecond)
	defer ticker.Stop()
	var lastErr error
	lastValue := int64(-1)
	for {
		var response querierv1.SelectMergeStacktracesResponse
		lastErr = query(ctx, client, url, "SelectMergeStacktraces", request, &response)
		if lastErr == nil && response.GetPprof().GetProfile() != nil {
			var data []byte
			data, lastErr = proto.Marshal(response.GetPprof().GetProfile())
			if lastErr == nil {
				var p *profile.Profile
				p, lastErr = profile.ParseData(data)
				if lastErr == nil {
					lastValue = markerValue(p, marker, kind)
					if got := lastValue; got >= want {
						if got != want {
							t.Fatalf("%s/%s: got %d, want %d", marker, kind, got, want)
						}
						return p
					}
					lastErr = fmt.Errorf("%s/%s: value %d, waiting for %d", marker, kind, markerValue(p, marker, kind), want)
				}
			}
		}
		select {
		case <-ctx.Done():
			t.Fatalf("%s/%s query timed out: %v; value=%d, want=%d; last result: %v", marker, kind, ctx.Err(), lastValue, want, lastErr)
		case <-ticker.C:
		}
	}
}

func query(ctx context.Context, client *http.Client, url, method string, request, response proto.Message) error {
	data, err := proto.Marshal(request)
	if err != nil {
		return err
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost,
		url+"/querier.v1.QuerierService/"+method, bytes.NewReader(data))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/proto")
	req.Header.Set("Accept-Encoding", "identity")
	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	const limit = 32 << 20
	body, err := io.ReadAll(io.LimitReader(resp.Body, limit+1))
	if err != nil {
		return err
	}
	if len(body) > limit {
		return fmt.Errorf("%s response exceeds %d bytes", method, limit)
	}
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("%s: HTTP %d: %.1024s", method, resp.StatusCode, body)
	}
	return proto.Unmarshal(body, response)
}

func markerValue(p *profile.Profile, marker, kind string) int64 {
	index := -1
	for i, typ := range p.SampleType {
		if typ.Type == kind {
			index = i
			break
		}
	}
	if index < 0 {
		return 0
	}
	var value int64
	for _, sample := range p.Sample {
		if sampleHasMarker(sample, marker) {
			value += sample.Value[index]
		}
	}
	return value
}

func sampleHasMarker(sample *profile.Sample, marker string) bool {
	for _, location := range sample.Location {
		for _, line := range location.Line {
			if line.Function != nil && strings.Contains(line.Function.Name, marker) {
				return true
			}
		}
	}
	return false
}

// Exclude recorder scaffolding, while keeping callers and their generic arguments.
func applicationFrames(p *profile.Profile) error {
	for _, f := range p.Function {
		owner := strings.TrimPrefix(f.Name, "<")
		if strings.HasPrefix(owner, "rustfs_mimalloc_profiling::profiler::") ||
			strings.HasPrefix(owner, "rustfs_mimalloc_profiling::memory_pprof::") ||
			strings.HasPrefix(owner, "backtrace::") ||
			strings.Contains(f.Name, "rustfs_mimalloc_profiling::profiler::SamplerState>") ||
			strings.Contains(f.Name, "rustfs_mimalloc_profiling::profiler::RegisteredTlsSampleBuffer>") {
			return fmt.Errorf("unexpected profiler frame %q", f.Name)
		}
	}
	return nil
}

func sourceMetadata(p *profile.Profile, marker string, uploaded bool) error {
	for _, location := range p.Location {
		for _, line := range location.Line {
			f := line.Function
			if f == nil || !strings.Contains(f.Name, marker) {
				continue
			}
			if !strings.HasSuffix(f.Filename, "mimalloc_server.rs") || f.SystemName == "" || line.Line <= 0 {
				return fmt.Errorf("missing source metadata for %s", marker)
			}
			if runtime.GOOS == "linux" || runtime.GOOS == "darwin" {
				if location.Mapping == nil || location.Mapping.BuildID == "" {
					return fmt.Errorf("missing executable build ID for %s", marker)
				}
			}
			// The server clears addresses for fully symbolized mappings.
			if uploaded && location.Address == 0 {
				return fmt.Errorf("missing uploaded address for %s", marker)
			}
			return nil
		}
	}
	return fmt.Errorf("application marker %s not found", marker)
}

func saveProfile(t *testing.T, directory, name string, p *profile.Profile) {
	t.Helper()
	var data bytes.Buffer
	if err := p.WriteUncompressed(&data); err != nil {
		t.Fatal(err)
	}
	saveArtifact(t, directory, name, data.Bytes())
}

func saveJSON(t *testing.T, directory, name string, message proto.Message) {
	t.Helper()
	data, err := (protojson.MarshalOptions{Indent: "  "}).Marshal(message)
	if err != nil {
		t.Fatal(err)
	}
	saveArtifact(t, directory, name, data)
}

func saveArtifact(t *testing.T, directory, name string, data []byte) {
	t.Helper()
	if err := os.WriteFile(filepath.Join(directory, name), data, 0o644); err != nil {
		t.Fatal(err)
	}
}
