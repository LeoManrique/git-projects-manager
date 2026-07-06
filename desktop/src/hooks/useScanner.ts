import { useState, useRef, useEffect, useCallback } from 'react';
import { api } from '../lib/api';
import { MonitoredFolder, ScanResult, RepoStatus } from '../types';
import { repoName } from '../lib/repoUtils';

// Minimum interval between silent scans (FRONTEND.md §5.1)
const SILENT_SCAN_MIN_INTERVAL_MS = 20_000;

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
 * Owns all scan state and repo operations (FRONTEND.md §5): full / per-folder /
 * silent scans with supersession, auto-scan on startup, focus rescan, and the
 * pull / clean actions with their automatic full rescan.
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

  const performScan = useCallback(async (foldersToScan: MonitoredFolder[]) => {
    const scans = foldersToScan.map(async (folder) => {
      try {
        const result = await api.scanFolder(folder.path, folder.onlyLocalChecks);
        return { id: folder.id, result };
      } catch (err) {
        // Folder keeps its previous result silently (FRONTEND.md §5.1).
        console.error(`Scan failed for ${folder.path}:`, err);
        return { id: folder.id, result: null };
      }
    });
    const settled = await Promise.all(scans);
    const newResults: Record<string, ScanResult> = {};
    for (const { id, result } of settled) {
      if (result) newResults[id] = result;
    }
    return newResults;
  }, []);

  /**
   * On-demand scan with UI indicators. Full scans bump the version; a scan
   * whose version is stale on completion discards its results (§5.2).
   */
  const scan = useCallback(
    async (foldersToScan: MonitoredFolder[], isFullScan: boolean) => {
      if (foldersToScan.length === 0) return;
      let version = scanVersionRef.current;
      if (isFullScan) {
        version = ++scanVersionRef.current;
        setIsFullScanning(true);
      }
      setError('');
      setScanningFolders((prev) => {
        const next = new Set(prev);
        for (const folder of foldersToScan) next.add(folder.id);
        return next;
      });

      try {
        const newResults = await performScan(foldersToScan);
        lastScanTimeRef.current = Date.now();

        // Superseded by a newer full scan: discard; the newer scan owns the
        // in-progress indicators and will clear them when it completes.
        if (version !== scanVersionRef.current) return;

        setResults((prev) => ({ ...prev, ...newResults }));
      } catch (err) {
        console.error(err);
        if (version === scanVersionRef.current) setError('Failed to scan folder(s)');
      } finally {
        if (version === scanVersionRef.current) {
          setScanningFolders((prev) => {
            const next = new Set(prev);
            for (const folder of foldersToScan) next.delete(folder.id);
            return next;
          });
          if (isFullScan) setIsFullScanning(false);
        }
      }
    },
    [performScan]
  );

  /** Background rescan with no indicators, throttled to one per 20s (§5.1). */
  const silentScan = useCallback(
    async (foldersToScan: MonitoredFolder[]) => {
      if (foldersToScan.length === 0) return;
      if (Date.now() - lastScanTimeRef.current < SILENT_SCAN_MIN_INTERVAL_MS) return;

      const version = scanVersionRef.current;
      try {
        const newResults = await performScan(foldersToScan);
        lastScanTimeRef.current = Date.now();
        // Never stomp a visible scan that started meanwhile.
        if (version !== scanVersionRef.current) return;
        setResults((prev) => ({ ...prev, ...newResults }));
      } catch (err) {
        console.error('Silent scan failed:', err);
      }
    },
    [performScan]
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

  // Auto-scan all folders the first time the list becomes non-empty (§3).
  useEffect(() => {
    if (folders.length > 0 && !hasInitialScanRef.current) {
      hasInitialScanRef.current = true;
      void scan(folders, true);
    }
  }, [folders, scan]);

  // Silent rescan when the window regains focus (§5.1).
  useEffect(() => {
    const handleFocus = () => {
      if (hasInitialScanRef.current && folders.length > 0) {
        void silentScan(folders);
      }
    };
    window.addEventListener('focus', handleFocus);
    return () => window.removeEventListener('focus', handleFocus);
  }, [folders, silentScan]);

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
        void scan(folders, true);
      } catch (err) {
        setError(`Failed to pull ${repoPath}: ${err}`);
      } finally {
        withRepoFlag(setPullingRepos, [repoPath], false);
      }
    },
    [scan, folders]
  );

  const clean = useCallback(
    async (repoPath: string) => {
      withRepoFlag(setCleaningRepos, [repoPath], true);
      try {
        const result = await api.cleanRepo(repoPath);
        if (result.filesRemoved.length + result.directoriesRemoved.length === 0) {
          setError(`No ignored files to clean in ${repoName(repoPath)}`);
        }
        void scan(folders, true);
      } catch (err) {
        setError(`Failed to clean ${repoPath}: ${err}`);
      } finally {
        withRepoFlag(setCleaningRepos, [repoPath], false);
      }
    },
    [scan, folders]
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
        const failed = outcomes.filter((err) => err !== null).length;
        if (failed > 0) setError(`Failed to pull ${failed} repo(s)`);
        void scan(folders, true);
      } finally {
        withRepoFlag(setPullingRepos, paths, false);
        setIsBulkPulling(false);
      }
    },
    [scan, folders]
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
        const failed = outcomes.filter((err) => err !== null).length;
        if (failed > 0) setError(`Failed to clean ${failed} repo(s)`);
        void scan(folders, true);
      } finally {
        withRepoFlag(setCleaningRepos, paths, false);
        setIsBulkCleaning(false);
      }
    },
    [scan, folders]
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
