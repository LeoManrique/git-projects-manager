package main

import (
	"context"
	"log"
	"os"

	"git-projects-manager/services"
)

// App struct
type App struct {
	ctx              context.Context
	configManager    *services.ConfigManager
	gitRepository    *services.GitRepository
}

// NewApp creates a new App application struct
func NewApp() *App {
	return &App{}
}

// startup is called when the app starts. The context is saved
// so we can call the runtime methods
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx

	// Initialize services
	configManager, err := services.NewConfigManager()
	if err != nil {
		log.Printf("Failed to initialize config manager: %v", err)
		os.Exit(1)
	}

	a.configManager = configManager
	a.gitRepository = services.NewGitRepository()
}

// GetMonitoredFolders returns all monitored folders
func (a *App) GetMonitoredFolders() []services.MonitoredFolder {
	return a.configManager.GetFolders()
}

// AddMonitoredFolder adds a new monitored folder
func (a *App) AddMonitoredFolder(path, name string) (services.MonitoredFolder, error) {
	return a.configManager.AddFolder(path, name)
}

// UpdateMonitoredFolder updates an existing monitored folder
func (a *App) UpdateMonitoredFolder(id, path, name string) (services.MonitoredFolder, error) {
	return a.configManager.UpdateFolder(id, path, name)
}

// DeleteMonitoredFolder removes a monitored folder
func (a *App) DeleteMonitoredFolder(id string) error {
	return a.configManager.DeleteFolder(id)
}

// ScanFolder scans a folder for git repositories
func (a *App) ScanFolder(path string) services.ScanResult {
	return a.gitRepository.ScanDirectory(path)
}
