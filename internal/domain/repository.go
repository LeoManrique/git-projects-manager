package domain

// RepoStatus holds the status of a git repository
type RepoStatus struct {
	Path         string `json:"path"`
	Branch       string `json:"branch,omitempty"`
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
