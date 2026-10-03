import { useState, useRef, useEffect, useCallback } from 'react';
import { api } from '../lib/api';
import { MonitoredFolder, ScanResult, RepoStatus } from '../types';
import { repoName } from '../lib/repoUtils';
import { log, logError } from '../lib/log';

// Minimum interval between focus-triggered rescans (FRONTEND.md §5.1)
const FOCUS_SCAN_MIN_INTERVAL_MS = 20_000;

/** True when `repoPath` is `folderPath` or sits underneath it. */
function isInside(repoPath: string, folderPath: string): boolean {
  const base = folderPath.replace(/[/\\]+$/, '');
  if (repoPath === base) return true;
  if (!repoPath.startsWith(base)) return false;
  const separator = repoPath[base.length];
  return separator === '/' || separator === '\\';
}

export interface UseScannerReturn {
  results: Record<string, ScanResult>;
  scanningFolders: Set<string>;
  isFullScanning: boolean;
  error: string;
  setError: (message: string) => void;
  pullingRepos: Set<string>;
  cleaningRepos: Set<string>;
  isBulkPulling: boolean;
  isBulkCleaning: boolean;
  scanAll: () => void;
  scanFolder: (folder: MonitoredFolder) => void;
  pull: (repoPath: string) => Promise<void>;
  clean: (repoPath: string) => Promise<void>;
  pullAll: (repos: RepoStatus[]) => Promise<void>;
  cleanAll: (repos: RepoStatus[]) => Promise<void>;
}

/**
 * Owns all scan state and repo operations (FRONTEND.md §5): full / per-folder
 * scans with supersession, auto-scan on startup, the throttled focus rescan
 * (a full scan, so it shows the normal indicators), and the pull / clean
 * actions with their automatic rescan of the affected folders.
 */
export function useScanner(folders: MonitoredFolder[]): UseScannerReturn {
  const [results, setResults] = useState<Record<string, ScanResult>>({});
  const [scanningFolders, setScanningFolders] = useState<Set<string>>(new Set());
  const [isFullScanning, setIsFullScanning] = useState(false);
  const [error, setError] = useState('');
  const [pullingRepos, setPullingRepos] = useState<Set<string>>(new Set());
  const [cleaningRepos, setCleaningRepos] = useState<Set<string>>(new Set());
  const [isBulkPulling, setIsBulkPulling] = useState(false);
  const [isBulkCleaning, setIsBulkCleaning] = useState(false);

  const scanVersionRef = useRef(0);
  const lastScanTimeRef = useRef(0);
  const hasInitialScanRef = useRef(false);
  // The ids monitored right now, read by scans that started before a folder
  // was deleted.
  const folderIdsRef = useRef(new Set<string>());

  // Every message the shared error surface shows also goes to the log: the
  // banner is replaced by the next message, so it cannot be the only record
  // (FRONTEND.md §6.4).
  useEffect(() => {
    if (error) log('warn', `banner: ${error}`);
  }, [error]);

  // Drop the scan state of folders that are no longer monitored (§5.2). A
  // folder deleted mid-scan used to stay in `scanningFolders` forever once a
  // full scan superseded its own, and a non-empty set turns off the focus
  // rescan for the rest of the session.
  useEffect(() => {
    const ids = new Set(folders.map((f) => f.id));
    folderIdsRef.current = ids;
    setScanningFolders((prev) => {
      const next = new Set([...prev].filter((id) => ids.has(id)));
      return next.size === prev.size ? prev : next;
    });
    setResults((prev) => {
      const kept = Object.entries(prev).filter(([id]) => ids.has(id));
      return kept.length === Object.keys(prev).length ? prev : Object.fromEntries(kept);
    });
  }, [folders]);

  /**
   * Scan with UI indicators. Full scans bump the version; a folder whose
   * version is stale on completion discards its result (§5.2).
   *
   * `clearError` is false for the rescans that follow a pull/clean, so the
   * action's own message survives the scan it triggers. Per §5.6 the error
   * surface clears when the next *on-demand* scan starts.
   */
  const scan = useCallback(
    async (foldersToScan: MonitoredFolder[], isFullScan: boolean, clearError = true) => {
      if (foldersToScan.length === 0) return;
      let version = scanVersionRef.current;
      if (isFullScan) {
        version = ++scanVersionRef.current;
        setIsFullScanning(true);
      }
      if (clearError) setError('');
      setScanningFolders((prev) => {
        const next = new Set(prev);
        for (const folder of foldersToScan) next.add(folder.id);
        return next;
      });

      // Each folder merges into the results map as it completes (§5.1) rather
      // than the whole batch landing at once behind the slowest folder.
      await Promise.all(
        foldersToScan.map(async (folder) => {
          const result = await api
            .scanFolder(folder, () => {})
            .catch((err: unknown) => {
              // Folder keeps its previous result silently (§5.1), so the log
              // is the only place this shows up.
              logError(`Scan failed for ${folder.path}`, err);
              return null;
            });

          // Superseded by a newer full scan: discard, and leave the indicators
          // to the scan that owns them now (§5.2).
          if (version !== scanVersionRef.current) return;

          // A folder deleted while it was scanning must not get its result back.
          if (result && folderIdsRef.current.has(folder.id)) {
            setResults((prev) => ({ ...prev, [folder.id]: result }));
          }
          setScanningFolders((prev) => {
            const next = new Set(prev);
            next.delete(folder.id);
            return next;
          });
        })
      );

      lastScanTimeRef.current = Date.now();
      if (isFullScan && version === scanVersionRef.current) setIsFullScanning(false);
    },
    []
  );

  const scanAll = useCallback(() => {
    void scan(folders, true);
  }, [scan, folders]);

  const scanFolder = useCallback(
    (folder: MonitoredFolder) => {
      void scan([folder], false);
    },
    [scan]
  );

  /**
   * The monitored folders that actually contain the given repos. Pulling or
   * cleaning a repo cannot change any other folder's state, so rescanning all
   * of them was one full network scan per action. Falls back to every folder
   * when a repo can't be attributed, so a miss is never a missed refresh.
   */
  const foldersForRepos = useCallback(
    (repoPaths: string[]) => {
      const affected = new Map<string, MonitoredFolder>();
      for (const repoPath of repoPaths) {
        // Longest match wins, so nested monitored folders attribute correctly.
        let best: MonitoredFolder | undefined;
        for (const folder of folders) {
          if (!isInside(repoPath, folder.path)) continue;
          if (!best || folder.path.length > best.path.length) best = folder;
        }
        if (best) affected.set(best.id, best);
      }
      return affected.size > 0 ? [...affected.values()] : folders;
    },
    [folders]
  );

  // Auto-scan all folders the first time the list becomes non-empty (§3).
  useEffect(() => {
    if (folders.length > 0 && !hasInitialScanRef.current) {
      hasInitialScanRef.current = true;
      void scan(folders, true);
    }
  }, [folders, scan]);

  // Rescan when the window regains focus: a normal full scan (so it shows the
  // usual global + per-folder indicators), throttled to once per 20s and
  // skipped while a scan is already in flight (§5.1).
  useEffect(() => {
    const handleFocus = () => {
      if (!hasInitialScanRef.current || folders.length === 0) return;
      if (isFullScanning || scanningFolders.size > 0) return;
      if (Date.now() - lastScanTimeRef.current < FOCUS_SCAN_MIN_INTERVAL_MS) return;
      void scan(folders, true);
    };
    window.addEventListener('focus', handleFocus);
    return () => window.removeEventListener('focus', handleFocus);
  }, [folders, scan, isFullScanning, scanningFolders]);

  const withRepoFlag = (
    setFlagged: React.Dispatch<React.SetStateAction<Set<string>>>,
    paths: string[],
    flagged: boolean
  ) => {
    setFlagged((prev) => {
      const next = new Set(prev);
      for (const path of paths) {
        if (flagged) next.add(path);
        else next.delete(path);
      }
      return next;
    });
  };

  const pull = useCallback(
    async (repoPath: string) => {
      withRepoFlag(setPullingRepos, [repoPath], true);
      try {
        await api.pullRepo(repoPath);
        void scan(foldersForRepos([repoPath]), false, false);
      } catch (err) {
        setError(`Failed to pull ${repoPath}: ${err}`);
      } finally {
        withRepoFlag(setPullingRepos, [repoPath], false);
      }
    },
    [scan, foldersForRepos]
  );

  const clean = useCallback(
    async (repoPath: string) => {
      withRepoFlag(setCleaningRepos, [repoPath], true);
      try {
        const result = await api.cleanRepo(repoPath);
        if (result.filesRemoved.length + result.directoriesRemoved.length === 0) {
          setError(`No ignored files to clean in ${repoName(repoPath)}`);
        }
        void scan(foldersForRepos([repoPath]), false, false);
      } catch (err) {
        setError(`Failed to clean ${repoPath}: ${err}`);
      } finally {
        withRepoFlag(setCleaningRepos, [repoPath], false);
      }
    },
    [scan, foldersForRepos]
  );

  const pullAll = useCallback(
    async (repos: RepoStatus[]) => {
      if (repos.length === 0) return;
      const paths = repos.map((r) => r.path);
      setIsBulkPulling(true);
      withRepoFlag(setPullingRepos, paths, true);
      try {
        const outcomes = await Promise.all(
          paths.map((path) => api.pullRepo(path).then(() => null, (err) => err))
        );
        const failures = outcomes.filter((err) => err !== null);
        // The first reason is included: a bare count told the user nothing
        // about which repo failed or why.
        if (failures.length > 0) {
          setError(`Failed to pull ${failures.length} repo(s): ${failures[0]}`);
        }
        void scan(foldersForRepos(paths), false, false);
      } finally {
        withRepoFlag(setPullingRepos, paths, false);
        setIsBulkPulling(false);
      }
    },
    [scan, foldersForRepos]
  );

  const cleanAll = useCallback(
    async (repos: RepoStatus[]) => {
      if (repos.length === 0) return;
      const paths = repos.map((r) => r.path);
      setIsBulkCleaning(true);
      withRepoFlag(setCleaningRepos, paths, true);
      try {
        const outcomes = await Promise.all(
          paths.map((path) => api.cleanRepo(path).then(() => null, (err) => err))
        );
        const failures = outcomes.filter((err) => err !== null);
        if (failures.length > 0) {
          setError(`Failed to clean ${failures.length} repo(s): ${failures[0]}`);
        }
        void scan(foldersForRepos(paths), false, false);
      } finally {
        withRepoFlag(setCleaningRepos, paths, false);
        setIsBulkCleaning(false);
      }
    },
    [scan, foldersForRepos]
  );

  return {
    results,
    scanningFolders,
    isFullScanning,
    error,
    setError,
    pullingRepos,
    cleaningRepos,
    isBulkPulling,
    isBulkCleaning,
    scanAll,
    scanFolder,
    pull,
    clean,
    pullAll,
    cleanAll,
  };
}
