//go:build windows
// +build windows

package services

import (
	"os/exec"
	"syscall"
)

// hideConsoleWindow hides the console window on Windows when executing commands
func hideConsoleWindow(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{
		HideWindow: true,
	}
}
