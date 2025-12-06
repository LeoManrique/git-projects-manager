package git

import (
	"git-projects-manager/internal/domain"
	"os"
	"path/filepath"
	"sort"
	"sync"
	"time"
)

// Scanner handles scanning directories for git repositories
type Scanner struct {
	repo *Repository
}

// NewScanner creates a new Scanner instance
func NewScanner() *Scanner {
	return &Scanner{
		repo: New(),
	}
}

// shouldSkipFolder checks if a folder should be excluded from scanning
func shouldSkipFolder(path string) bool {
	folderName := filepath.Base(path)
	return ExcludedFolders[folderName]
}

// ScanDirectory scans a directory for git repositories and returns their status
func (s *Scanner) ScanDirectory(rootDir string) domain.ScanResult {
	startTime := time.Now()

	// Channel for collecting repo statuses
	statusChan := make(chan domain.RepoStatus, 100)
	var wg sync.WaitGroup

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
		if s.repo.IsGitRepo(path) {
			// Spawn goroutine to check status concurrently
			wg.Add(1)
			go func(repoPath string) {
				defer wg.Done()

				relPath, _ := filepath.Rel(rootDir, repoPath)
				branch := s.repo.GetCurrentBranch(repoPath)
				hasChanges := s.repo.HasPendingChanges(repoPath)
				hasUnpushed := s.repo.HasUnpushedCommits(repoPath)

				status := domain.RepoStatus{
					Path:        relPath,
					Branch:      branch,
					HasChanges:  hasChanges,
					HasUnpushed: hasUnpushed,
					HasError:    false,
				}

				if hasChanges == nil && hasUnpushed == nil {
					status.HasError = true
					status.ErrorMessage = "Failed to check repository status"
				}

				statusChan <- status
			}(path)

			return filepath.SkipDir
		}

		return nil
	})

	// Close channel once all goroutines are done
	go func() {
		wg.Wait()
		close(statusChan)
	}()

	// Collect results
	reposWithChanges := []domain.RepoStatus{}
	reposWithUnpushed := []domain.RepoStatus{}
	reposClean := []domain.RepoStatus{}
	reposError := []domain.RepoStatus{}
	uniqueRepos := make(map[string]bool)

	for status := range statusChan {
		uniqueRepos[status.Path] = true

		if status.HasError {
			reposError = append(reposError, status)
		} else if (status.HasChanges != nil && *status.HasChanges) || (status.HasUnpushed != nil && *status.HasUnpushed) {
			if status.HasChanges != nil && *status.HasChanges {
				reposWithChanges = append(reposWithChanges, status)
			}
			if status.HasUnpushed != nil && *status.HasUnpushed {
				reposWithUnpushed = append(reposWithUnpushed, status)
			}
		} else {
			reposClean = append(reposClean, status)
		}
	}

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

	elapsedTime := time.Since(startTime)

	return domain.ScanResult{
		ScannedPath:       rootDir,
		TotalRepositories: len(uniqueRepos),
		WithChanges:       reposWithChanges,
		WithUnpushed:      reposWithUnpushed,
		Clean:             reposClean,
		Errors:            reposError,
		ExecutionTime:     elapsedTime.Seconds(),
	}
}
