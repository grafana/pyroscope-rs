package e2e

import (
	"context"
	"os/exec"
	"time"
)

func commandContext(ctx context.Context, program string, args ...string) *exec.Cmd {
	cmd := exec.CommandContext(ctx, program, args...)
	// A descendant may inherit output pipes. Bound draining even if it
	// escapes the process group or group cancellation is unavailable.
	cmd.WaitDelay = 5 * time.Second
	isolateCommand(cmd)
	return cmd
}
