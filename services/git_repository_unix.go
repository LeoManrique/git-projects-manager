//go:build !windows
// +build !windows

package services

import "os/exec"

// hideConsoleWindow is a no-op on Unix-like systems
func hideConsoleWindow(cmd *exec.Cmd) {
	// No console window to hide on Unix systems
}
