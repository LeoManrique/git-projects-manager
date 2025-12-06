package app

import (
	"context"
	"git-projects-manager/internal/config"
	"git-projects-manager/internal/domain"
	"git-projects-manager/internal/git"
	"log"
	"os"

	"github.com/wailsapp/wails/v2/pkg/runtime"
)

// App struct
type App struct {
	ctx           context.Context
	configManager *config.Manager
	gitScanner    *git.Scanner
}

// New creates a new App application struct
func New() *App {
	return &App{}
}

// Startup is called when the app starts. The context is saved
// so we can call the runtime methods
func (a *App) Startup(ctx context.Context) {
	a.ctx = ctx

	// Initialize services
	configManager, err := config.NewManager()
	if err != nil {
		log.Printf("Failed to initialize config manager: %v", err)
		os.Exit(1)
	}

	a.configManager = configManager
	a.gitScanner = git.NewScanner()
}

// GetMonitoredFolders returns all monitored folders
func (a *App) GetMonitoredFolders() []domain.MonitoredFolder {
	return a.configManager.GetFolders()
}

// AddMonitoredFolder adds a new monitored folder
func (a *App) AddMonitoredFolder(path, name string) (domain.MonitoredFolder, error) {
	return a.configManager.AddFolder(path, name)
}

// UpdateMonitoredFolder updates an existing monitored folder
func (a *App) UpdateMonitoredFolder(id, path, name string) (domain.MonitoredFolder, error) {
	return a.configManager.UpdateFolder(id, path, name)
}

// DeleteMonitoredFolder removes a monitored folder
func (a *App) DeleteMonitoredFolder(id string) error {
	return a.configManager.DeleteFolder(id)
}

// ScanFolder scans a folder for git repositories
func (a *App) ScanFolder(path string) domain.ScanResult {
	return a.gitScanner.ScanDirectory(path)
}

// BrowseFolder opens a directory picker dialog and returns the selected path
func (a *App) BrowseFolder() (string, error) {
	return runtime.OpenDirectoryDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select Folder",
	})
}
