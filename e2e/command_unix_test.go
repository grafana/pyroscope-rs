//go:build unix

package e2e

import (
	"context"
	"io"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"
)

func isolateCommand(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Cancel = func() error {
		// Cargo can launch rustc, linkers and the Rust fixture itself.
		// Killing its group prevents those children from outliving the test.
		err := syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		if err == syscall.ESRCH {
			return os.ErrProcessDone
		}
		return err
	}
}

type commandReadyWriter struct {
	ready chan struct{}
	once  sync.Once
}

func (w *commandReadyWriter) Write(p []byte) (int, error) {
	w.once.Do(func() { close(w.ready) })
	return len(p), nil
}

func TestCommandCancellationReleasesDescendantPipes(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	pidFile := t.TempDir() + "/child.pid"
	cmd := commandContext(ctx, "sh", "-c",
		"sleep 30 & printf '%s\\n' \"$!\" > \"$1\"; printf ready; wait", "fixture", pidFile)
	ready := &commandReadyWriter{ready: make(chan struct{})}
	cmd.Stdout = ready
	cmd.Stderr = io.Discard
	monitor, inherited, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer monitor.Close()
	defer inherited.Close()
	// This descriptor observes child lifetime independently of os/exec's
	// stdout pipes, which WaitDelay is allowed to close.
	cmd.ExtraFiles = []*os.File{inherited}
	descendantsExited := false
	t.Cleanup(func() {
		cancel()
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
		if descendantsExited {
			return
		}
		// Also clean up when testing a broken implementation that leaves
		// the descendant alive, so the regression test itself cannot leak.
		if data, err := os.ReadFile(pidFile); err == nil {
			if pid, err := strconv.Atoi(strings.TrimSpace(string(data))); err == nil {
				if process, err := os.FindProcess(pid); err == nil {
					_ = process.Kill()
				}
			}
		}
	})
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	inherited.Close()
	waited := make(chan error, 1)
	go func() { waited <- cmd.Wait() }()
	closed := make(chan error, 1)
	go func() {
		_, err := io.Copy(io.Discard, monitor)
		closed <- err
	}()
	select {
	case <-ready.ready:
	case <-ctx.Done():
		t.Fatal("child did not become ready:", ctx.Err())
	}
	cancel()
	select {
	case err := <-waited:
		if err == nil {
			t.Fatal("cancelled command reported success")
		}
	case <-time.After(2 * time.Second):
		t.Fatal("cancellation left Wait blocked on descendant output pipes")
	}
	select {
	case err := <-closed:
		if err != nil {
			t.Fatal(err)
		}
		descendantsExited = true
	case <-time.After(2 * time.Second):
		t.Fatal("cancellation left a descendant holding the inherited descriptor")
	}
}
