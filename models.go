package main

// RepoStatus holds the status of a git repository
type RepoStatus struct {
	Path           string `json:"path"`
	HasChanges     *bool  `json:"hasChanges"`
	HasUnpushed    *bool  `json:"hasUnpushed"`
	HasError       bool   `json:"hasError"`
	ErrorMessage   string `json:"errorMessage,omitempty"`
}

// ScanResult contains the results of scanning a directory
type ScanResult struct {
	ScannedPath      string       `json:"scannedPath"`
	TotalRepositories int         `json:"totalRepositories"`
	WithChanges      []RepoStatus `json:"withChanges"`
	WithUnpushed     []RepoStatus `json:"withUnpushed"`
	Clean            []RepoStatus `json:"clean"`
	Errors           []RepoStatus `json:"errors"`
	ExecutionTime    float64      `json:"executionTime"`
}

// MonitoredFolder represents a folder to be monitored
type MonitoredFolder struct {
	ID    string `json:"id"`
	Path  string `json:"path"`
	Name  string `json:"name"`
	IsWSL bool   `json:"isWSL"`
}

// Config holds the application configuration
type Config struct {
	Folders []MonitoredFolder `json:"folders"`
}
