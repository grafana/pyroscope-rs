//go:build !unix

package e2e

import "os/exec"

// Other platforms retain CommandContext's process cancellation and WaitDelay.
func isolateCommand(_ *exec.Cmd) {}
