package e2e

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

const pyroscopeImage = "grafana/pyroscope:2.4.0@sha256:b7e62699a27463aa3533efe3f88c320da594da495731fd6eabca6f44f92cdd7b"

// Go owns the container lifecycle and all E2E assertions. Rust only produces
// allocator samples and uploads them through the SDK.
// CI selects one provider subtest per job; make mimalloc/test/server runs both.
func TestMimallocServer(t *testing.T) {
	for _, provider := range []string{"upstream", "rustfs"} {
		t.Run(provider, func(t *testing.T) {
			testMimallocServer(t, provider)
		})
	}
}

func testMimallocServer(t *testing.T, provider string) {
	signals, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	ctx, cancel := context.WithTimeout(signals, 10*time.Minute)
	defer cancel()
	working, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	root := filepath.Dir(working)
	artifacts := os.Getenv("PYROSCOPE_TEST_ARTIFACT_DIR")
	if artifacts == "" {
		artifacts = filepath.Join(root, "target", "mimalloc-server-e2e")
	} else if !filepath.IsAbs(artifacts) {
		artifacts = filepath.Join(root, artifacts)
	}
	if err := os.MkdirAll(artifacts, 0o755); err != nil {
		t.Fatal(err)
	}
	// Each run gets fresh inputs, so stale artifacts cannot hide a broken fixture.
	artifacts, err = os.MkdirTemp(artifacts, provider+"-")
	if err != nil {
		t.Fatal(err)
	}
	t.Logf("artifacts: %s", artifacts)
	image := os.Getenv("PYROSCOPE_TEST_IMAGE")
	if image == "" {
		image = pyroscopeImage
	}
	revision := output(t, ctx, root, "git", "rev-parse", "HEAD")
	dirty := output(t, ctx, root, "git", "status", "--porcelain") != ""
	metadata := fmt.Sprintf("image=%s\nwrite_path=segment-writer\nsource_revision=%s\nworktree_dirty=%t\nallocator_provider=%s\n", image, revision, dirty, provider)
	if err := os.WriteFile(filepath.Join(artifacts, "server.env"), []byte(metadata), 0o644); err != nil {
		t.Fatal(err)
	}
	container := fmt.Sprintf("pyroscope-mimalloc-%s-e2e-%d-%d", provider, os.Getpid(), time.Now().UnixNano())
	started := false
	// Register before starting Docker so a partially failed start is cleaned up.
	t.Cleanup(func() {
		logs, logErr := cleanupDocker("logs", container)
		if err := os.WriteFile(filepath.Join(artifacts, "server.log"), logs, 0o644); err != nil {
			t.Errorf("save server logs: %v", err)
		}
		if logErr != nil && started {
			t.Errorf("collect server logs: %v", logErr)
		}
		if result, err := cleanupDocker("rm", "--force", container); err != nil && started {
			t.Errorf("remove server: %v: %s", err, result)
		}
	})
	output(t, ctx, root, "docker", "run", "--detach", "--name", container,
		"--cpus", "2", "--memory", "1g", "--workdir", "/data",
		"--tmpfs", "/data:rw,size=256m,mode=1777", "-p", "127.0.0.1::4040", image,
		"-target=all", "-write-path=segment-writer", "-enable-query-backend-from=1970-01-01T00:00:00Z",
		"-log.level=debug", "-usage-stats.enabled=false", "-self-profiling.disable-push=true",
		"-segment-writer.min-ready-duration=0s", "-metastore.min-ready-duration=0s",
		"-pyroscopedb.data-path=/data/v1", "-storage.filesystem.dir=/data/shared")
	started = true
	url := "http://" + output(t, ctx, root, "docker", "port", container, "4040/tcp")
	ready, readyCancel := context.WithTimeout(ctx, 90*time.Second)
	defer readyCancel()
	client := &http.Client{Timeout: time.Second}
	for {
		request, err := http.NewRequestWithContext(ready, http.MethodGet, url+"/ready", nil)
		if err != nil {
			t.Fatal(err)
		}
		response, err := client.Do(request)
		if err == nil {
			response.Body.Close()
			if response.StatusCode == http.StatusOK {
				break
			}
		}
		if ready.Err() != nil {
			t.Fatalf("server did not become ready: %v", ready.Err())
		}
		if output(t, ready, root, "docker", "inspect", "--format", "{{.State.Running}}", container) != "true" {
			t.Fatal("server exited before readiness; see server.log")
		}
		select {
		case <-ready.Done():
			t.Fatalf("server did not become ready: %v", ready.Err())
		case <-time.After(time.Second):
		}
	}
	log, err := os.Create(filepath.Join(artifacts, "test.log"))
	if err != nil {
		t.Fatal(err)
	}
	defer log.Close()
	args := []string{"test", "--locked", "--manifest-path",
		filepath.Join(root, "e2e", "fixtures", provider, "Cargo.toml"),
		"--test", "mimalloc_server", "--", "--ignored", "--exact", "fixture::upload_mimalloc_fixture", "--nocapture"}
	command := exec.CommandContext(ctx, "cargo", args...)
	command.Dir = root
	command.Env = append(os.Environ(), "PYROSCOPE_TEST_URL="+url,
		"PYROSCOPE_TEST_SERVICE="+container, "PYROSCOPE_TEST_ARTIFACT_DIR="+artifacts)
	if _, configured := os.LookupEnv("CARGO_TARGET_DIR"); !configured {
		command.Env = append(command.Env, "CARGO_TARGET_DIR="+filepath.Join(root, "target"))
	}
	command.Stdout = io.MultiWriter(os.Stdout, log)
	command.Stderr = command.Stdout
	if err := command.Run(); err != nil {
		t.Fatalf("Rust upload fixture: %v; see test.log and server.log", err)
	}
	verifyMimallocProfiles(t, ctx, url, container, artifacts)
}

// Log collection must not consume the timeout needed to remove the container.
func cleanupDocker(args ...string) ([]byte, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	return exec.CommandContext(ctx, "docker", args...).CombinedOutput()
}

func output(t *testing.T, ctx context.Context, directory, program string, args ...string) string {
	t.Helper()
	command := exec.CommandContext(ctx, program, args...)
	command.Dir = directory
	result, err := command.CombinedOutput()
	if err != nil {
		t.Fatalf("%s %v: %v\n%s", program, args, err, result)
	}
	return strings.TrimSpace(string(result))
}
