package git

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// Repository handles git repository operations
type Repository struct{}

// New creates a new Repository instance
func New() *Repository {
	return &Repository{}
}

// IsGitRepo checks if a directory is a git repository
func (r *Repository) IsGitRepo(path string) bool {
	gitPath := filepath.Join(path, ".git")
	info, err := os.Stat(gitPath)
	return err == nil && info.IsDir()
}

// GetCurrentBranch gets the current branch name of a git repository
func (r *Repository) GetCurrentBranch(repoPath string) string {
	cmd := exec.Command("git", "rev-parse", "--abbrev-ref", "HEAD")
	cmd.Dir = repoPath
	hideConsoleWindow(cmd)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(output))
}

// HasPendingChanges checks if a git repository has pending changes
func (r *Repository) HasPendingChanges(repoPath string) *bool {
	cmd := exec.Command("git", "status", "--porcelain")
	cmd.Dir = repoPath
	hideConsoleWindow(cmd)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil
	}
	hasChanges := strings.TrimSpace(string(output)) != ""
	return &hasChanges
}

// HasUnpushedCommits checks if a git repository has unpushed commits
func (r *Repository) HasUnpushedCommits(repoPath string) *bool {
	// Get current branch
	cmd := exec.Command("git", "rev-parse", "--abbrev-ref", "HEAD")
	cmd.Dir = repoPath
	hideConsoleWindow(cmd)
	branchOutput, err := cmd.CombinedOutput()
	if err != nil {
		return nil
	}
	branch := strings.TrimSpace(string(branchOutput))

	// Check if branch has upstream
	cmd = exec.Command("git", "rev-parse", "--abbrev-ref", branch+"@{upstream}")
	cmd.Dir = repoPath
	hideConsoleWindow(cmd)
	_, err = cmd.CombinedOutput()
	if err != nil {
		return nil // No upstream configured
	}

	// Check for unpushed commits
	cmd = exec.Command("git", "log", "@{upstream}..HEAD", "--oneline")
	cmd.Dir = repoPath
	hideConsoleWindow(cmd)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil
	}
	hasUnpushed := strings.TrimSpace(string(output)) != ""
	return &hasUnpushed
}
