import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
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

/** A folder's scan in flight, and whether it shows its progress. */
interface InFlightScan {
  done: Promise<void>;
  visible: boolean;
}

export interface UseScannerReturn {
  results: Record<string, ScanResult>;
  /** Folders with a visible scan in flight. */
  scanningFolders: Set<string>;
  /** Repos a visible scan has not checked yet; their rows show a spinner. */
  checkingRepos: Set<string>;
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
 * Owns all scan state and repo operations (FRONTEND.md §5): streamed scans,
 * at most one per folder (a second request joins it), auto-scan on startup,
 * the throttled focus rescan, and the pull / clean actions with their
 * recheck of the repos they touched.
 */
export function useScanner(folders: MonitoredFolder[]): UseScannerReturn {
  const [results, setResults] = useState<Record<string, ScanResult>>({});
  const [scanningFolders, setScanningFolders] = useState<Set<string>>(new Set());
  // Scan All presses (and other visible full scans) still running.
  const [visibleFullScans, setVisibleFullScans] = useState(0);
  const [error, setError] = useState('');
  const [pullingRepos, setPullingRepos] = useState<Set<string>>(new Set());
  const [cleaningRepos, setCleaningRepos] = useState<Set<string>>(new Set());
  const [isBulkPulling, setIsBulkPulling] = useState(false);
  const [isBulkCleaning, setIsBulkCleaning] = useState(false);

  // A ref, not state, so a request sees a scan started a moment earlier.
  // `scanningFolders` mirrors its visible entries for rendering (§5.2).
  const inFlightRef = useRef(new Map<string, InFlightScan>());
  const queuedSnapshotsRef = useRef(new Map<string, ScanResult>());
  const flushFrameRef = useRef<number | null>(null);
  const lastScanTimeRef = useRef(0);
  const hasInitialScanRef = useRef(false);
  // The folders monitored right now, read by scans and actions that started
  // before a folder was edited or deleted.
  const foldersRef = useRef(folders);

  // Every message the shared error surface shows also goes to the log: the
  // banner is replaced by the next message, so it cannot be the only record
  // (FRONTEND.md §6.4).
  useEffect(() => {
    if (error) log('warn', `banner: ${error}`);
  }, [error]);

  // Drop the results of folders that are no longer monitored (§5.2). A scan
  // of one still in flight clears its own entry when it ends.
  useEffect(() => {
    const ids = new Set(folders.map((f) => f.id));
    foldersRef.current = folders;
    setResults((prev) => {
      const kept = Object.entries(prev).filter(([id]) => ids.has(id));
      return kept.length === Object.keys(prev).length ? prev : Object.fromEntries(kept);
    });
  }, [folders]);

  const checkingRepos = useMemo(
    () => new Set([...scanningFolders].flatMap((id) => results[id]?.pending ?? [])),
    [scanningFolders, results]
  );

  /**
   * Show each snapshot as its folder's result, unless the folder is gone or
   * what is shown is already as new.
   */
  const applySnapshots = useCallback((snapshots: Map<string, ScanResult>) => {
    setResults((prev) => {
      let next = prev;
      for (const [folderId, snapshot] of snapshots) {
        if (!foldersRef.current.some((f) => f.id === folderId)) continue;
        const shown = next[folderId];
        if (shown && shown.revision >= snapshot.revision) continue;
        if (next === prev) next = { ...prev };
        next[folderId] = snapshot;
      }
      return next;
    });
  }, []);

  /**
   * Streamed snapshots land once per frame, the newest per folder, so repos
   * finishing in a burst render once.
   */
  const queueSnapshot = useCallback(
    (folderId: string, snapshot: ScanResult) => {
      queuedSnapshotsRef.current.set(folderId, snapshot);
      if (flushFrameRef.current !== null) return;
      flushFrameRef.current = requestAnimationFrame(() => {
        flushFrameRef.current = null;
        const snapshots = queuedSnapshotsRef.current;
        queuedSnapshotsRef.current = new Map();
        applySnapshots(snapshots);
      });
    },
    [applySnapshots]
  );

  const publishInFlight = useCallback(() => {
    const visible = [...inFlightRef.current].filter(([, scan]) => scan.visible).map(([id]) => id);
    setScanningFolders((prev) =>
      prev.size === visible.length && visible.every((id) => prev.has(id)) ? prev : new Set(visible)
    );
  }, []);

  /** One folder's scan, streaming its snapshots onto the screen. Never rejects. */
  const runScan = useCallback(
    async (folder: MonitoredFolder) => {
      try {
        const final = await api.scanFolder(folder, (snapshot) => queueSnapshot(folder.id, snapshot));
        applySnapshots(new Map([[folder.id, final]]));
      } catch (err) {
        // The folder keeps what it had (§5.1), so the log is the only place
        // this shows up.
        logError(`Scan failed for ${folder.path}`, err);
      } finally {
        inFlightRef.current.delete(folder.id);
        publishInFlight();
      }
    },
    [queueSnapshot, applySnapshots, publishInFlight]
  );

  /**
   * Scan `targets` concurrently and wait for all of them (§5.1–5.2). The
   * single entry point behind every trigger: a folder already scanning is
   * joined instead of scanned twice, and turns visible when this request is.
   * A visible scan shows its progress; a silent one only its results.
   */
  const requestScan = useCallback(
    async (targets: MonitoredFolder[], visible: boolean) => {
      lastScanTimeRef.current = Date.now();
      const scans = targets.map((folder) => {
        const inFlight = inFlightRef.current.get(folder.id);
        if (inFlight) {
          if (visible) inFlight.visible = true;
          return inFlight.done;
        }
        // `runScan` removes the entry only after an await, so it is in place first.
        const done = runScan(folder);
        inFlightRef.current.set(folder.id, { done, visible });
        return done;
      });
      publishInFlight();
      await Promise.all(scans);
    },
    [runScan, publishInFlight]
  );

  /** On-demand, so it clears the shared error surface (§5.6). */
  const fullScan = useCallback(async () => {
    setError('');
    setVisibleFullScans((n) => n + 1);
    await requestScan(folders, true);
    setVisibleFullScans((n) => n - 1);
  }, [requestScan, folders]);

  const scanAll = useCallback(() => {
    void fullScan();
  }, [fullScan]);

  const scanFolder = useCallback(
    (folder: MonitoredFolder) => {
      setError('');
      void requestScan([folder], true);
    },
    [requestScan]
  );

  /**
   * `repoPaths` grouped by the monitored folder that contains them. A repo no
   * folder contains is left out: it belongs to no result on screen.
   */
  const reposByFolder = useCallback((repoPaths: string[]) => {
    const groups = new Map<string, { folder: MonitoredFolder; repos: string[] }>();
    for (const repoPath of repoPaths) {
      // Longest match wins, so nested monitored folders attribute correctly.
      let best: MonitoredFolder | undefined;
      for (const folder of foldersRef.current) {
        if (!isInside(repoPath, folder.path)) continue;
        if (!best || folder.path.length > best.path.length) best = folder;
      }
      if (!best) continue;
      const group = groups.get(best.id) ?? { folder: best, repos: [] };
      group.repos.push(repoPath);
      groups.set(best.id, group);
    }
    return [...groups.values()];
  }, []);

  /**
   * Read `repoPaths` again after an action changed them, each folder's in one
   * call (§5.5). Never starts a scan, and never clears the message the action
   * just set (§5.6). A folder the core has not scanned since its path was last
   * set returns nothing, and keeps what it shows. Never rejects.
   */
  const recheck = useCallback(
    async (repoPaths: string[]) => {
      await Promise.all(
        reposByFolder(repoPaths).map(async ({ folder, repos }) => {
          try {
            const snapshot = await api.recheckRepos(folder, repos);
            if (snapshot) applySnapshots(new Map([[folder.id, snapshot]]));
          } catch (err) {
            logError(`Recheck failed in ${folder.path}`, err);
          }
        })
      );
    },
    [reposByFolder, applySnapshots]
  );

  // Auto-scan all folders the first time the list becomes non-empty (§3).
  useEffect(() => {
    if (folders.length > 0 && !hasInitialScanRef.current) {
      hasInitialScanRef.current = true;
      void fullScan();
    }
  }, [folders, fullScan]);

  // Rescan when the window regains focus: a normal full scan (so it shows the
  // usual global + per-folder indicators), throttled to once per 20s and
  // skipped while any scan is in flight (§5.1).
  useEffect(() => {
    const handleFocus = () => {
      if (!hasInitialScanRef.current || folders.length === 0) return;
      if (inFlightRef.current.size > 0) return;
      if (Date.now() - lastScanTimeRef.current < FOCUS_SCAN_MIN_INTERVAL_MS) return;
      void fullScan();
    };
    window.addEventListener('focus', handleFocus);
    return () => window.removeEventListener('focus', handleFocus);
  }, [folders, fullScan]);

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

  // Every action rechecks its repos, failed or not (a pull can fail after its
  // fetch moved the counts), and keeps them flagged until the recheck lands,
  // so the row spins from the click until it shows the new state.
  const pull = useCallback(
    async (repoPath: string) => {
      withRepoFlag(setPullingRepos, [repoPath], true);
      try {
        await api.pullRepo(repoPath);
      } catch (err) {
        setError(`Failed to pull ${repoPath}: ${err}`);
      }
      await recheck([repoPath]);
      withRepoFlag(setPullingRepos, [repoPath], false);
    },
    [recheck]
  );

  const clean = useCallback(
    async (repoPath: string) => {
      withRepoFlag(setCleaningRepos, [repoPath], true);
      try {
        const result = await api.cleanRepo(repoPath);
        if (result.filesRemoved.length + result.directoriesRemoved.length === 0) {
          setError(`No ignored files to clean in ${repoName(repoPath)}`);
        }
      } catch (err) {
        setError(`Failed to clean ${repoPath}: ${err}`);
      }
      await recheck([repoPath]);
      withRepoFlag(setCleaningRepos, [repoPath], false);
    },
    [recheck]
  );

  const pullAll = useCallback(
    async (repos: RepoStatus[]) => {
      if (repos.length === 0) return;
      const paths = repos.map((r) => r.path);
      setIsBulkPulling(true);
      withRepoFlag(setPullingRepos, paths, true);
      const outcomes = await Promise.all(
        paths.map((path) => api.pullRepo(path).then(() => null, (err) => err))
      );
      const failures = outcomes.filter((err) => err !== null);
      // The first reason is included: a bare count told the user nothing
      // about which repo failed or why.
      if (failures.length > 0) {
        setError(`Failed to pull ${failures.length} repo(s): ${failures[0]}`);
      }
      await recheck(paths);
      withRepoFlag(setPullingRepos, paths, false);
      setIsBulkPulling(false);
    },
    [recheck]
  );

  const cleanAll = useCallback(
    async (repos: RepoStatus[]) => {
      if (repos.length === 0) return;
      const paths = repos.map((r) => r.path);
      setIsBulkCleaning(true);
      withRepoFlag(setCleaningRepos, paths, true);
      const outcomes = await Promise.all(
        paths.map((path) => api.cleanRepo(path).then(() => null, (err) => err))
      );
      const failures = outcomes.filter((err) => err !== null);
      if (failures.length > 0) {
        setError(`Failed to clean ${failures.length} repo(s): ${failures[0]}`);
      }
      await recheck(paths);
      withRepoFlag(setCleaningRepos, paths, false);
      setIsBulkCleaning(false);
    },
    [recheck]
  );

  return {
    results,
    scanningFolders,
    checkingRepos,
    isFullScanning: visibleFullScans > 0,
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
