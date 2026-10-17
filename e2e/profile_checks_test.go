package e2e

import (
	"context"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/google/pprof/profile"
	querierv1 "github.com/grafana/pyroscope/api/gen/proto/go/querier/v1"
)

func TestMarkerValueCountsEachSampleOnce(t *testing.T) {
	f := &profile.Function{Name: liveMarker}
	l := &profile.Location{Line: []profile.Line{{Function: f}, {Function: f}}}
	p := &profile.Profile{
		SampleType: []*profile.ValueType{{Type: "alloc_space"}, {Type: "inuse_space"}},
		Sample: []*profile.Sample{
			{Location: []*profile.Location{l, l}, Value: []int64{100, 20}},
			{Location: []*profile.Location{l}, Value: []int64{200, 30}},
			{Value: []int64{999, 999}},
		},
	}
	if got := markerValue(p, liveMarker, "inuse_space"); got != 50 {
		t.Fatalf("got %d, want 50; repeated marker frames must not duplicate sample values", got)
	}
	if got := markerValue(p, freedControlMarker, "inuse_space"); got != 0 {
		t.Fatalf("absent marker: got %d, want 0", got)
	}
	if got := markerValue(p, liveMarker, "inuse_objects"); got != 0 {
		t.Fatalf("absent sample type: got %d, want 0", got)
	}
}

func TestSourceMetadataChecksUploadsAndNormalizedQueries(t *testing.T) {
	f := &profile.Function{Name: liveMarker, SystemName: "mangled", Filename: "mimalloc_server.rs"}
	l := &profile.Location{
		Address: 42, Mapping: &profile.Mapping{BuildID: "test-build"},
		Line: []profile.Line{{Function: f, Line: 12}},
	}
	p := &profile.Profile{Location: []*profile.Location{l}}
	if err := sourceMetadata(p, liveMarker, true); err != nil {
		t.Fatal(err)
	}
	l.Address = 0
	if err := sourceMetadata(p, liveMarker, true); err == nil {
		t.Fatal("uploaded profile without an address was accepted")
	}
	if err := sourceMetadata(p, liveMarker, false); err != nil {
		t.Fatalf("normalized server profile: %v", err)
	}
	l.Line[0].Line = 0
	if err := sourceMetadata(p, liveMarker, false); err == nil {
		t.Fatal("profile without a source line was accepted")
	}
	if err := sourceMetadata(p, freedControlMarker, false); err == nil {
		t.Fatal("profile without the application marker was accepted")
	}
}

func TestQueryRejectsHTTPFailureAndHonorsCancellation(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		http.Error(w, "unavailable", http.StatusServiceUnavailable)
	}))
	defer server.Close()
	request := &querierv1.ProfileTypesRequest{}
	response := &querierv1.ProfileTypesResponse{}
	if err := query(context.Background(), server.Client(), server.URL, "ProfileTypes", request, response); err == nil || !strings.Contains(err.Error(), "503") {
		t.Fatalf("expected an HTTP 503 error, got %v", err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if err := query(ctx, server.Client(), server.URL, "ProfileTypes", request, response); !errors.Is(err, context.Canceled) {
		t.Fatalf("expected context cancellation, got %v", err)
	}
}
