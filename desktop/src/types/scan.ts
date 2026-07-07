export interface RepoStatus {
  path: string;
  branch?: string;
  hasChanges?: boolean;
  hasUnpushed?: boolean;
  hasUnpulled?: boolean;
  /** Whether a remote is configured; false means never published. */
  hasRemote?: boolean;
  hasError: boolean;
  errorMessage?: string;
}

export interface ScanResult {
  scannedPath: string;
  totalRepositories: number;
  withChanges: RepoStatus[];
  withUnpushed: RepoStatus[];
  withUnpulled: RepoStatus[];
  /** No remote configured (never published). Overlaps other buckets. */
  unpublished: RepoStatus[];
  clean: RepoStatus[];
  errors: RepoStatus[];
  uninitialized: RepoStatus[];
  executionTime: number;
}
