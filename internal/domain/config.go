package domain

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
