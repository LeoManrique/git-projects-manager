import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { api } from '../lib/api';
import { MonitoredFolder, ScanResult, RepoStatus } from '../types';
import { repoName } from '../lib/repoUtils';
import { log, logError } from '../lib/log';

// The schedule (FRONTEND.md §5.1): the "Last scan" clock is the debouncer.
// A silent full scan runs once the last full scan is this old.
const BACKGROUND_SCAN_MIN_AGE_MS = 10 * 60_000;
// Coming to the front, or the window showing again, runs a visible one past this.
const FOCUS_SCAN_MIN_AGE_MS = 15 * 60_000;
// WebKit's timers stop while the system sleeps, so a long timeout would fire
// as late as the sleep was long: the background timer wakes at least this often.
const MAX_TIMER_MS = 60_000;
// A background timer this late was held back by system sleep (or a suspended
// WebView) and likely fired before the network is back: it waits first.
const LATE_TIMER_MS = 60_000;
const WAKE_GRACE_MS = 30_000;

/** Whether a scan of `a` also answers for `b`: the name plays no part. */
function scansAlike(a: MonitoredFolder, b: MonitoredFolder): boolean {
  return (
    a.path === b.path &&
    a.onlyLocalChecks === b.onlyLocalChecks &&
    a.detectUninitialized === b.detectUninitialized
  );
}

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
  id: symbol;
  /** The folder as it was when the scan was asked for. */
  folder: MonitoredFolder;
  /** Resolves to the scan's start (unix ms), or null if it failed. */
  done: Promise<number | null>;
  visible: boolean;
}

export interface UseScannerReturn {
  results: Record<string, ScanResult>;
  /** Folders with a visible scan in flight. */
  scanningFolders: Set<string>;
  /** Repos a visible scan has not checked yet; their rows show a spinner. */
  checkingRepos: Set<string>;
  isFullScanning: boolean;
  /** When the last full scan started (unix ms): what "Last scan" shows. */
  lastFullScanStartedAt: number | null;
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
 * the background and focus rescans, and the pull / clean actions with their
 * recheck of the repos they touched.
 */
export function useScanner(folders: MonitoredFolder[]): UseScannerReturn {
  const [results, setResults] = useState<Record<string, ScanResult>>({});
  const [scanningFolders, setScanningFolders] = useState<Set<string>>(new Set());
  // Scan All presses (and other visible full scans) still running.
  const [visibleFullScans, setVisibleFullScans] = useState(0);
  const [lastFullScanStartedAt, setLastFullScanStartedAt] = useState<number | null>(null);
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
  // `lastFullScanStartedAt` for the timer and listeners, which outlive renders.
  const lastFullScanRef = useRef<number | null>(null);
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

  /**
   * One folder's scan, streaming its snapshots onto the screen. Resolves to
   * when the scan started, or null if it failed. Never rejects.
   */
  const runScan = useCallback(
    async (folder: MonitoredFolder) => {
      try {
        const final = await api.scanFolder(folder, (snapshot) => queueSnapshot(folder.id, snapshot));
        applySnapshots(new Map([[folder.id, final]]));
        // Unfinished only when the folder was edited or deleted meanwhile.
        return final.isComplete ? final.startedAtMs : null;
      } catch (err) {
        // The folder keeps what it had (§5.1), so the log is the only place
        // this shows up.
        logError(`Scan failed for ${folder.path}`, err);
        return null;
      }
    },
    [queueSnapshot, applySnapshots]
  );

  /**
   * Scan `targets` concurrently and wait for all of them (§5.1–5.2). The
   * single entry point behind every trigger: a folder already scanning is
   * joined instead of scanned twice, and turns visible when this request is.
   * A visible scan shows its progress; a silent one only its results.
   * Resolves to when each target's scan started, null for one that failed.
   */
  const requestScan = useCallback(
    async (targets: MonitoredFolder[], visible: boolean) => {
      const scans = targets.map((folder) => {
        const inFlight = inFlightRef.current.get(folder.id);
        if (inFlight && scansAlike(inFlight.folder, folder)) {
          if (visible) inFlight.visible = true;
          return inFlight.done;
        }
        // A scan of the folder as it was before an edit is not joined: this
        // one waits for it, so the two never fetch a repo at once. The entry
        // is removed only after an await, so it is in place first.
        const id = Symbol(folder.path);
        const done = (async () => {
          await inFlight?.done;
          try {
            return await runScan(folder);
          } finally {
            // A newer scan of the edited folder may have taken the entry already.
            if (inFlightRef.current.get(folder.id)?.id === id) inFlightRef.current.delete(folder.id);
            publishInFlight();
          }
        })();
        inFlightRef.current.set(folder.id, { id, folder, done, visible });
        return done;
      });
      publishInFlight();
      return Promise.all(scans);
    },
    [runScan, publishInFlight]
  );

  /**
   * Move the "Last scan" clock to the start of a full scan that just ended:
   * the earliest of its folders' scans, joined ones included, so the label
   * never claims a folder is fresher than it is. A folder whose scan failed
   * does not hold it back. It moves back only from a start in the future,
   * left by a system clock set back since.
   */
  const recordFullScan = useCallback((starts: (number | null)[]) => {
    const known = starts.filter((start) => start !== null);
    if (known.length === 0) return;
    const earliest = Math.min(...known);
    const current = lastFullScanRef.current;
    if (current !== null && earliest < current && current <= Date.now()) return;
    lastFullScanRef.current = earliest;
    setLastFullScanStartedAt(earliest);
  }, []);

  /**
   * How long until the last full scan is `ageMs` old: 0 once it is, and when
   * its age is unknown (no full scan yet, or a start in the future after the
   * system clock was set back).
   */
  const timeUntilLastFullScanIsOlderThan = useCallback((ageMs: number) => {
    const last = lastFullScanRef.current;
    if (last === null) return 0;
    const elapsed = Date.now() - last;
    return elapsed < 0 ? 0 : Math.max(ageMs - elapsed, 0);
  }, []);

  /** A scan of every folder, which moves the "Last scan" clock once it ends. */
  const fullScan = useCallback(
    async (visible: boolean) => {
      if (visible) setVisibleFullScans((n) => n + 1);
      recordFullScan(await requestScan(foldersRef.current, visible));
      if (visible) setVisibleFullScans((n) => n - 1);
    },
    [requestScan, recordFullScan]
  );

  /**
   * Visible full scan: Scan All, the startup auto-scan and the focus rescan.
   * On-demand, so it clears the shared error surface (§5.6).
   */
  const scanAll = useCallback(() => {
    setError('');
    void fullScan(true);
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
      scanAll();
    }
  }, [folders, scanAll]);

  // The window came to the front or shows again: rescan every folder, visibly,
  // if the last full scan is FOCUS_SCAN_MIN_AGE_MS old (§5.1). A scan in
  // flight is joined, and turns visible.
  useEffect(() => {
    const scanIfStale = () => {
      if (document.visibilityState === 'hidden') return;
      if (!hasInitialScanRef.current || foldersRef.current.length === 0) return;
      if (timeUntilLastFullScanIsOlderThan(FOCUS_SCAN_MIN_AGE_MS) > 0) return;
      scanAll();
    };
    window.addEventListener('focus', scanIfStale);
    document.addEventListener('visibilitychange', scanIfStale);
    return () => {
      window.removeEventListener('focus', scanIfStale);
      document.removeEventListener('visibilitychange', scanIfStale);
    };
  }, [scanAll, timeUntilLastFullScanIsOlderThan]);

  // Silent full scans for as long as the app runs, each once the last full
  // scan is BACKGROUND_SCAN_MIN_AGE_MS old, whatever started that one (§5.1).
  // The window always exists: closing it quits the app.
  useEffect(() => {
    let stopped = false;
    let timer: number | undefined;
    const sleep = (ms: number) =>
      new Promise<void>((resolve) => {
        timer = window.setTimeout(resolve, ms);
      });
    // After a run that moved nothing (no folder, or every scan failed), look
    // again in an interval rather than at once.
    let retryAt = 0;
    const untilDue = () =>
      Math.max(timeUntilLastFullScanIsOlderThan(BACKGROUND_SCAN_MIN_AGE_MS), retryAt - Date.now(), 0);
    void (async () => {
      while (!stopped) {
        const delay = Math.min(untilDue(), MAX_TIMER_MS);
        const firesAt = Date.now() + delay;
        await sleep(delay);
        if (Date.now() - firesAt > LATE_TIMER_MS) await sleep(WAKE_GRACE_MS);
        // A scan that ended meanwhile moved the clock: sleep again.
        if (stopped || untilDue() > 0) continue;
        await fullScan(false);
        if (timeUntilLastFullScanIsOlderThan(BACKGROUND_SCAN_MIN_AGE_MS) === 0) {
          retryAt = Date.now() + BACKGROUND_SCAN_MIN_AGE_MS;
        }
      }
    })();
    // The loop's pending sleep never resolves, so it stops there.
    return () => {
      stopped = true;
      window.clearTimeout(timer);
    };
  }, [fullScan, timeUntilLastFullScanIsOlderThan]);

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
    lastFullScanStartedAt,
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
