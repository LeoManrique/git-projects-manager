package config

import (
	"encoding/json"
	"git-projects-manager/internal/domain"
	"os"
	"path/filepath"
	"runtime"

	"github.com/google/uuid"
)

// Manager handles configuration persistence
type Manager struct {
	configPath string
	config     *domain.Config
}

// NewManager creates a new Manager instance
func NewManager() (*Manager, error) {
	configDir, err := getConfigDirectory()
	if err != nil {
		return nil, err
	}

	configPath := filepath.Join(configDir, "git-projects-manager-config.json")

	m := &Manager{
		configPath: configPath,
		config:     &domain.Config{Folders: []domain.MonitoredFolder{}},
	}

	// Load existing config if it exists
	_ = m.Load()

	// Save empty config if none exists
	if len(m.config.Folders) == 0 {
		_ = m.Save()
	}

	return m, nil
}

// getConfigDirectory returns the appropriate config directory based on OS
func getConfigDirectory() (string, error) {
	var configDir string

	switch runtime.GOOS {
	case "windows":
		configDir = os.Getenv("APPDATA")
		if configDir == "" {
			configDir = os.Getenv("USERPROFILE")
		}
	case "darwin", "linux":
		configDir = os.Getenv("HOME")
	}

	if configDir == "" {
		configDir = "."
	}

	appConfigDir := filepath.Join(configDir, ".git-projects-manager")
	if err := os.MkdirAll(appConfigDir, 0700); err != nil {
		return "", err
	}

	return appConfigDir, nil
}

// Load loads the configuration from disk
func (m *Manager) Load() error {
	data, err := os.ReadFile(m.configPath)
	if err != nil {
		return err
	}

	if err := json.Unmarshal(data, m.config); err != nil {
		return err
	}

	return nil
}

// Save persists the configuration to disk
func (m *Manager) Save() error {
	data, err := json.MarshalIndent(m.config, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(m.configPath, data, 0644)
}

// GetFolders returns all monitored folders
func (m *Manager) GetFolders() []domain.MonitoredFolder {
	return m.config.Folders
}

// AddFolder adds a new monitored folder
func (m *Manager) AddFolder(path, name string) (domain.MonitoredFolder, error) {
	folder := domain.MonitoredFolder{
		ID:   uuid.New().String(),
		Path: path,
		Name: name,
	}

	m.config.Folders = append(m.config.Folders, folder)
	if err := m.Save(); err != nil {
		return domain.MonitoredFolder{}, err
	}

	return folder, nil
}

// UpdateFolder updates an existing monitored folder
func (m *Manager) UpdateFolder(id, path, name string) (domain.MonitoredFolder, error) {
	for i, folder := range m.config.Folders {
		if folder.ID == id {
			m.config.Folders[i] = domain.MonitoredFolder{
				ID:   id,
				Path: path,
				Name: name,
			}
			if err := m.Save(); err != nil {
				return domain.MonitoredFolder{}, err
			}
			return m.config.Folders[i], nil
		}
	}

	return domain.MonitoredFolder{}, os.ErrNotExist
}

// DeleteFolder removes a monitored folder
func (m *Manager) DeleteFolder(id string) error {
	for i, folder := range m.config.Folders {
		if folder.ID == id {
			m.config.Folders = append(m.config.Folders[:i], m.config.Folders[i+1:]...)
			return m.Save()
		}
	}

	return os.ErrNotExist
}
