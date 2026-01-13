export interface RepoStatus {
  path: string;
  branch?: string;
  hasChanges?: boolean;
  hasUnpushed?: boolean;
  hasUnpulled?: boolean;
  hasError: boolean;
  errorMessage?: string;
}

export interface ScanResult {
  scannedPath: string;
  totalRepositories: number;
  withChanges: RepoStatus[];
  withUnpushed: RepoStatus[];
  withUnpulled: RepoStatus[];
  clean: RepoStatus[];
  errors: RepoStatus[];
  uninitialized: RepoStatus[];
  executionTime: number;
}
