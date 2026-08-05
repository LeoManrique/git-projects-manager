/**
 * Where a repo stands relative to a remote host (mirrors core `PublishState`).
 * `remoteNotFound` means a remote is configured but the host reports it is gone.
 */
export type PublishState = 'published' | 'unpublished' | 'remoteNotFound';

export interface RepoStatus {
  path: string;
  branch?: string;
  hasChanges?: boolean;
  hasUnpushed?: boolean;
  hasUnpulled?: boolean;
  /** Publish state relative to the remote host. */
  publishState: PublishState;
  hasError: boolean;
  errorMessage?: string;
}

export interface ScanResult {
  scannedPath: string;
  totalRepositories: number;
  withChanges: RepoStatus[];
  withUnpushed: RepoStatus[];
  withUnpulled: RepoStatus[];
  /** No remote configured (never published). Overlay: overlaps other buckets. */
  unpublished: RepoStatus[];
  /** Remote configured but gone on the host. Overlay: overlaps other buckets. */
  remoteNotFound: RepoStatus[];
  clean: RepoStatus[];
  errors: RepoStatus[];
  uninitialized: RepoStatus[];
  executionTime: number;
}
