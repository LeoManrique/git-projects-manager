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
  /**
   * The remote comparison was attempted and failed, so `hasUnpushed` and
   * `hasUnpulled` are unknown rather than false. Always false for folders with
   * `onlyLocalChecks` — there the scan never asked, which is not a failure.
   */
  remoteStateUnknown: boolean;
  /** Publish state relative to the remote host. */
  publishState: PublishState;
  hasError: boolean;
  errorMessage?: string;
}

/** One folder's state at one moment (mirrors core `ScanResult`). */
export interface ScanResult {
  scannedPath: string;
  /** Repos found by the latest walk, `checking` included. */
  totalRepositories: number;
  /** Wall clock at the start of the scan this snapshot belongs to, unix ms. */
  startedAtMs: number;
  /** Increases with every change, so an older snapshot arriving late can be dropped. */
  revision: number;
  /** Repos the scan in flight has not checked yet. */
  pending: string[];
  /** Repos found by this scan that have no status yet (path only). */
  checking: RepoStatus[];
  /** True once the scan has checked every repo it found. */
  isComplete: boolean;
  withChanges: RepoStatus[];
  withUnpushed: RepoStatus[];
  withUnpulled: RepoStatus[];
  /** No remote configured (never published). Overlay: overlaps other buckets. */
  unpublished: RepoStatus[];
  /** Remote configured but gone on the host. Overlay: overlaps other buckets. */
  remoteNotFound: RepoStatus[];
  /** Remote comparison attempted and failed. Overlay: overlaps other buckets. */
  remoteStateUnknown: RepoStatus[];
  clean: RepoStatus[];
  errors: RepoStatus[];
  uninitialized: RepoStatus[];
  /** Seconds the scan took. Zero until `isComplete`. */
  executionTime: number;
}
