package services

import (
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

// GitRepository handles git repository operations
type GitRepository struct{}

// NewGitRepository creates a new GitRepository instance
func NewGitRepository() *GitRepository {
	return &GitRepository{}
}

// shouldSkipFolder checks if a folder should be excluded from scanning
func shouldSkipFolder(path string) bool {
	folderName := filepath.Base(path)
	return ExcludedFolders[folderName]
}

// RepoStatus holds the status of a git repository
type RepoStatus struct {
	Path         string `json:"path"`
	HasChanges   *bool  `json:"hasChanges"`
	HasUnpushed  *bool  `json:"hasUnpushed"`
	HasError     bool   `json:"hasError"`
	ErrorMessage string `json:"errorMessage,omitempty"`
}

// ScanResult contains the results of scanning a directory
type ScanResult struct {
	ScannedPath       string       `json:"scannedPath"`
	TotalRepositories int          `json:"totalRepositories"`
	WithChanges       []RepoStatus `json:"withChanges"`
	WithUnpushed      []RepoStatus `json:"withUnpushed"`
	Clean             []RepoStatus `json:"clean"`
	Errors            []RepoStatus `json:"errors"`
	ExecutionTime     float64      `json:"executionTime"`
}

// IsGitRepo checks if a directory is a git repository
func (gr *GitRepository) IsGitRepo(path string) bool {
	gitPath := filepath.Join(path, ".git")
	info, err := os.Stat(gitPath)
	return err == nil && info.IsDir()
}

// HasPendingChanges checks if a git repository has pending changes
func (gr *GitRepository) HasPendingChanges(repoPath string) *bool {
	cmd := exec.Command("git", "status", "--porcelain")
	cmd.Dir = repoPath
	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil
	}
	hasChanges := strings.TrimSpace(string(output)) != ""
	return &hasChanges
}

// HasUnpushedCommits checks if a git repository has unpushed commits
func (gr *GitRepository) HasUnpushedCommits(repoPath string) *bool {
	// Get current branch
	cmd := exec.Command("git", "rev-parse", "--abbrev-ref", "HEAD")
	cmd.Dir = repoPath
	branchOutput, err := cmd.CombinedOutput()
	if err != nil {
		return nil
	}
	branch := strings.TrimSpace(string(branchOutput))

	// Check if branch has upstream
	cmd = exec.Command("git", "rev-parse", "--abbrev-ref", branch+"@{upstream}")
	cmd.Dir = repoPath
	_, err = cmd.CombinedOutput()
	if err != nil {
		return nil // No upstream configured
	}

	// Check for unpushed commits
	cmd = exec.Command("git", "log", "@{upstream}..HEAD", "--oneline")
	cmd.Dir = repoPath
	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil
	}
	hasUnpushed := strings.TrimSpace(string(output)) != ""
	return &hasUnpushed
}

// ScanDirectory scans a directory for git repositories and returns their status
func (gr *GitRepository) ScanDirectory(rootDir string) ScanResult {
	startTime := time.Now()

	reposWithChanges := []RepoStatus{}
	reposWithUnpushed := []RepoStatus{}
	reposClean := []RepoStatus{}
	reposError := []RepoStatus{}

	// Walk through all subdirectories
	filepath.Walk(rootDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}

		if !info.IsDir() {
			return nil
		}

		// Skip excluded folders
		if shouldSkipFolder(path) {
			return filepath.SkipDir
		}

		// Check if current directory is a git repo
		if gr.IsGitRepo(path) {
			relPath, _ := filepath.Rel(rootDir, path)
			hasChanges := gr.HasPendingChanges(path)
			hasUnpushed := gr.HasUnpushedCommits(path)

			status := RepoStatus{
				Path:        relPath,
				HasChanges:  hasChanges,
				HasUnpushed: hasUnpushed,
				HasError:    false,
			}

			if hasChanges == nil && hasUnpushed == nil {
				status.HasError = true
				status.ErrorMessage = "Failed to check repository status"
				reposError = append(reposError, status)
			} else if (hasChanges != nil && *hasChanges) || (hasUnpushed != nil && *hasUnpushed) {
				if hasChanges != nil && *hasChanges {
					reposWithChanges = append(reposWithChanges, status)
				}
				if hasUnpushed != nil && *hasUnpushed {
					reposWithUnpushed = append(reposWithUnpushed, status)
				}
			} else {
				reposClean = append(reposClean, status)
			}

			return filepath.SkipDir
		}

		return nil
	})

	// Sort results
	sort.Slice(reposWithChanges, func(i, j int) bool {
		return reposWithChanges[i].Path < reposWithChanges[j].Path
	})
	sort.Slice(reposWithUnpushed, func(i, j int) bool {
		return reposWithUnpushed[i].Path < reposWithUnpushed[j].Path
	})
	sort.Slice(reposClean, func(i, j int) bool {
		return reposClean[i].Path < reposClean[j].Path
	})
	sort.Slice(reposError, func(i, j int) bool {
		return reposError[i].Path < reposError[j].Path
	})

	// Calculate total unique repos
	uniqueRepos := make(map[string]bool)
	for _, r := range reposWithChanges {
		uniqueRepos[r.Path] = true
	}
	for _, r := range reposWithUnpushed {
		uniqueRepos[r.Path] = true
	}
	for _, r := range reposClean {
		uniqueRepos[r.Path] = true
	}
	for _, r := range reposError {
		uniqueRepos[r.Path] = true
	}

	elapsedTime := time.Since(startTime)

	return ScanResult{
		ScannedPath:       rootDir,
		TotalRepositories: len(uniqueRepos),
		WithChanges:       reposWithChanges,
		WithUnpushed:      reposWithUnpushed,
		Clean:             reposClean,
		Errors:            reposError,
		ExecutionTime:     elapsedTime.Seconds(),
	}
}
