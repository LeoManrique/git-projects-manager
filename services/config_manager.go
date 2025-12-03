package services

import (
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"

	"github.com/google/uuid"
)

// MonitoredFolder represents a folder to be monitored
type MonitoredFolder struct {
	ID   string `json:"id"`
	Path string `json:"path"`
	Name string `json:"name"`
}

// Config holds the application configuration
type Config struct {
	Folders []MonitoredFolder `json:"folders"`
}

// ConfigManager handles configuration persistence
type ConfigManager struct {
	configPath string
	config     *Config
}

// NewConfigManager creates a new ConfigManager instance
func NewConfigManager() (*ConfigManager, error) {
	configDir, err := getConfigDirectory()
	if err != nil {
		return nil, err
	}

	configPath := filepath.Join(configDir, "git-projects-manager-config.json")

	cm := &ConfigManager{
		configPath: configPath,
		config:     &Config{Folders: []MonitoredFolder{}},
	}

	// Load existing config if it exists
	_ = cm.Load()

	// Save empty config if none exists
	if len(cm.config.Folders) == 0 {
		_ = cm.Save()
	}

	return cm, nil
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
func (cm *ConfigManager) Load() error {
	data, err := os.ReadFile(cm.configPath)
	if err != nil {
		return err
	}

	if err := json.Unmarshal(data, cm.config); err != nil {
		return err
	}

	return nil
}

// Save persists the configuration to disk
func (cm *ConfigManager) Save() error {
	data, err := json.MarshalIndent(cm.config, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(cm.configPath, data, 0644)
}

// GetFolders returns all monitored folders
func (cm *ConfigManager) GetFolders() []MonitoredFolder {
	return cm.config.Folders
}

// AddFolder adds a new monitored folder
func (cm *ConfigManager) AddFolder(path, name string) (MonitoredFolder, error) {
	folder := MonitoredFolder{
		ID:   uuid.New().String(),
		Path: path,
		Name: name,
	}

	cm.config.Folders = append(cm.config.Folders, folder)
	if err := cm.Save(); err != nil {
		return MonitoredFolder{}, err
	}

	return folder, nil
}

// UpdateFolder updates an existing monitored folder
func (cm *ConfigManager) UpdateFolder(id, path, name string) (MonitoredFolder, error) {
	for i, folder := range cm.config.Folders {
		if folder.ID == id {
			cm.config.Folders[i] = MonitoredFolder{
				ID:   id,
				Path: path,
				Name: name,
			}
			if err := cm.Save(); err != nil {
				return MonitoredFolder{}, err
			}
			return cm.config.Folders[i], nil
		}
	}

	return MonitoredFolder{}, os.ErrNotExist
}

// DeleteFolder removes a monitored folder
func (cm *ConfigManager) DeleteFolder(id string) error {
	for i, folder := range cm.config.Folders {
		if folder.ID == id {
			cm.config.Folders = append(cm.config.Folders[:i], cm.config.Folders[i+1:]...)
			return cm.Save()
		}
	}

	return os.ErrNotExist
}
