import { useState, useRef, useEffect, useCallback } from 'react';
import { api } from '../lib/api';
import { MonitoredFolder, ScanResult, RepoStatus, TerminalApp, EditorApp } from '../types';
import { RepoSection, StatusBadge } from './scan';

// Minimum interval between silent scans (in milliseconds)
const SILENT_SCAN_MIN_INTERVAL_MS = 20_000; // 20 seconds

export interface ScanResultsState {
  results: Record<string, ScanResult>;
  expandedFolders: Set<string>;
}

interface ScanResultsProps {
  folders: MonitoredFolder[];
  scanState: ScanResultsState;
  onScanStateChange: React.Dispatch<React.SetStateAction<ScanResultsState>>;
  defaultTerminal: TerminalApp | null;
  defaultEditor: EditorApp | null;
  hasInitialScan: boolean;
  onInitialScanComplete: () => void;
}

export default function ScanResults({ folders, scanState, onScanStateChange, defaultTerminal, defaultEditor, hasInitialScan, onInitialScanComplete }: ScanResultsProps) {
  const { results, expandedFolders } = scanState;
  const [scanningFolders, setScanningFolders] = useState<Record<string, boolean>>({});
  const [isFullScanActive, setIsFullScanActive] = useState(false);
  const [error, setError] = useState('');
  const [pullingRepos, setPullingRepos] = useState<Set<string>>(new Set());
  const [cleaningRepos, setCleaningRepos] = useState<Set<string>>(new Set());
  const [isPullingAllUnpulled, setIsPullingAllUnpulled] = useState(false);
  const scanVersionRef = useRef(0);
  const lastScanTimeRef = useRef<number>(0);

  const handleOpenInTerminal = async (repoPath: string) => {
    if (!defaultTerminal) return;
    try {
      await api.openInTerminal(repoPath, defaultTerminal.id);
    } catch (err) {
      setError(`Failed to open terminal: ${err}`);
    }
  };

  const handleOpenInEditor = async (repoPath: string) => {
    if (!defaultEditor) return;
    try {
      await api.openInEditor(repoPath, defaultEditor.id);
    } catch (err) {
      setError(`Failed to open editor: ${err}`);
    }
  };

  const handlePull = async (repoPath: string) => {
    setPullingRepos(prev => new Set(prev).add(repoPath));
    try {
      await api.pullRepo(repoPath);
      scan(folders, true);
    } catch (err) {
      setError(`Failed to pull ${repoPath}: ${err}`);
    } finally {
      setPullingRepos(prev => {
        const next = new Set(prev);
        next.delete(repoPath);
        return next;
      });
    }
  };

  const handleClean = async (repoPath: string) => {
    setCleaningRepos(prev => new Set(prev).add(repoPath));
    try {
      const result = await api.cleanRepo(repoPath);
      const totalRemoved = result.filesRemoved.length + result.directoriesRemoved.length;
      if (totalRemoved === 0) {
        setError(`No ignored files to clean in ${repoPath.split('/').pop()}`);
      }
      // Refresh scan to update the status
      scan(folders, true);
    } catch (err) {
      setError(`Failed to clean ${repoPath}: ${err}`);
    } finally {
      setCleaningRepos(prev => {
        const next = new Set(prev);
        next.delete(repoPath);
        return next;
      });
    }
  };

  const handlePullAllUnpulled = async (unpulledRepos: RepoStatus[]) => {
    if (unpulledRepos.length === 0) return;

    setIsPullingAllUnpulled(true);
    const repoPaths = unpulledRepos.map(r => r.path);
    setPullingRepos(prev => new Set([...prev, ...repoPaths]));

    try {
      const pullPromises = repoPaths.map(path =>
        api.pullRepo(path).catch(err => ({ path, error: err }))
      );
      const results = await Promise.all(pullPromises);

      const errors = results.filter(r => r && typeof r === 'object' && 'error' in r);
      if (errors.length > 0) {
        setError(`Failed to pull ${errors.length} repo(s)`);
      }

      scan(folders, true);
    } catch (err) {
      setError(`Failed to pull repos: ${err}`);
    } finally {
      setPullingRepos(prev => {
        const next = new Set(prev);
        repoPaths.forEach(p => next.delete(p));
        return next;
      });
      setIsPullingAllUnpulled(false);
    }
  };

  const toggleExpandedFolder = (folderId: string) => {
    onScanStateChange(prev => {
      const newExpanded = new Set(prev.expandedFolders);
      if (newExpanded.has(folderId)) {
        newExpanded.delete(folderId);
      } else {
        newExpanded.add(folderId);
      }
      return { ...prev, expandedFolders: newExpanded };
    });
  };

  /**
   * Core scan logic - executes the actual folder scanning
   * Returns the scan results mapped by folder ID
   */
  const performScan = useCallback(async (foldersToScan: MonitoredFolder[]): Promise<Record<string, ScanResult>> => {
    const scanPromises = foldersToScan.map(async (folder) => {
      try {
        const result = await api.scanFolder(folder.path);
        return { id: folder.id, result, error: null };
      } catch (err) {
        return { id: folder.id, result: null, error: err };
      }
    });

    const scanResults = await Promise.allSettled(scanPromises);
    const newResults: Record<string, ScanResult> = {};

    for (const result of scanResults) {
      if (result.status === 'fulfilled' && result.value.result) {
        newResults[result.value.id] = result.value.result;
      }
    }

    return newResults;
  }, []);

  /**
   * Silent scan - runs in the background without UI indicators
   * Respects minimum interval between scans (considers both silent and on-demand scans)
   */
  const silentScan = useCallback(async (foldersToScan: MonitoredFolder[]) => {
    if (foldersToScan.length === 0) return;

    // Check minimum interval since last scan
    const now = Date.now();
    if (now - lastScanTimeRef.current < SILENT_SCAN_MIN_INTERVAL_MS) {
      return;
    }

    try {
      const newResults = await performScan(foldersToScan);
      lastScanTimeRef.current = Date.now();

      onScanStateChange(prev => ({
        ...prev,
        results: { ...prev.results, ...newResults }
      }));
    } catch (err) {
      console.error('Silent scan failed:', err);
    }
  }, [performScan, onScanStateChange]);

  /**
   * On-demand scan - shows UI indicators and always runs (no interval restriction)
   */
  const scan = async (foldersToScan: MonitoredFolder[], isFullScan: boolean = false) => {
    let currentVersion = scanVersionRef.current;
    if (isFullScan) {
      currentVersion = ++scanVersionRef.current;
      setIsFullScanActive(true);
    }

    try {
      setError('');

      setScanningFolders(prev => {
        const newState = { ...prev };
        for (const folder of foldersToScan) {
          newState[folder.id] = true;
        }
        return newState;
      });

      const newResults = await performScan(foldersToScan);
      lastScanTimeRef.current = Date.now();

      // Check if scan was superseded by a newer one
      if (currentVersion !== scanVersionRef.current) {
        setScanningFolders({});
        if (isFullScan) {
          setIsFullScanActive(false);
        }
        return;
      }

      const updatedFolderIds = new Set(Object.keys(newResults));

      onScanStateChange(prev => ({
        ...prev,
        results: { ...prev.results, ...newResults }
      }));

      if (isFullScan && foldersToScan.length === 1 && !expandedFolders.has(foldersToScan[0].id)) {
        toggleExpandedFolder(foldersToScan[0].id);
      }

      setScanningFolders(prev => {
        const newState = { ...prev };
        for (const folderId of updatedFolderIds) {
          delete newState[folderId];
        }
        return newState;
      });

      if (isFullScan) {
        setIsFullScanActive(false);
      }
    } catch (err) {
      setError('Failed to scan folder(s)');
      console.error(err);
      setScanningFolders(prev => {
        const newState = { ...prev };
        for (const folder of foldersToScan) {
          delete newState[folder.id];
        }
        return newState;
      });
      if (isFullScan) {
        setIsFullScanActive(false);
      }
    }
  };

  // Auto-scan all folders on app startup
  useEffect(() => {
    if (folders.length > 0 && !hasInitialScan) {
      onInitialScanComplete();
      scan(folders, true);
    }
  }, [folders, hasInitialScan, onInitialScanComplete]);

  // Silent scan when window regains focus
  useEffect(() => {
    const handleFocus = () => {
      // Only trigger if initial scan has completed and folders exist
      if (hasInitialScan && folders.length > 0) {
        silentScan(folders);
      }
    };

    window.addEventListener('focus', handleFocus);
    return () => window.removeEventListener('focus', handleFocus);
  }, [folders, silentScan, hasInitialScan]);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex-shrink-0 flex justify-between items-center px-3 py-2 border-b border-dark-border/50">
        <h2 className="text-sm font-medium text-text-primary">Repository Status</h2>
        <button
          onClick={() => scan(folders, true)}
          disabled={folders.length === 0}
          className="bg-accent-green/90 hover:bg-accent-green text-dark-bg text-xs font-medium py-1 px-2.5 rounded transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {isFullScanActive ? 'Scanning...' : 'Scan All'}
        </button>
      </div>

      {error && (
        <div className="flex-shrink-0 mx-3 mt-2 bg-accent-red/10 border border-accent-red/20 text-accent-red px-2.5 py-1.5 rounded text-xs">
          {error}
        </div>
      )}

      {folders.length === 0 ? (
        <div className="flex-1 flex items-center justify-center">
          <p className="text-text-muted text-xs">
            No folders configured. Add a folder to get started.
          </p>
        </div>
      ) : (
        <div className="flex-1 overflow-auto bg-dark-surface flex flex-col">
          {folders.map((folder, index) => {
            const result = results[folder.id];
            const isExpanded = expandedFolders.has(folder.id);
            const isFolderScanning = scanningFolders[folder.id] ?? false;
            const isLast = index === folders.length - 1;

            return (
              <div
                key={folder.id}
                className={`bg-dark-surface border-b border-dark-border flex flex-col ${isExpanded && isLast ? 'flex-1' : ''}`}
              >
                <button
                  onClick={() => toggleExpandedFolder(folder.id)}
                  className="w-full flex justify-between items-center px-2.5 py-2 hover:bg-dark-elevated/50 transition-colors text-left"
                >
                  <div className="flex-1 min-w-0">
                    <h3 className="text-text-primary text-xs font-medium truncate">{folder.name}</h3>
                    <p className="text-text-muted text-xs truncate font-mono mt-0.5">{folder.path}</p>
                  </div>
                  <div className="text-right mx-2 flex-shrink-0">
                    {isFolderScanning ? (
                      <span className="text-text-muted text-xs">Scanning...</span>
                    ) : result ? (
                      <div className="text-xs flex items-center gap-1.5">
                        <span className="text-text-secondary mr-0.5">
                          {result.totalRepositories} repos
                        </span>
                        <StatusBadge count={result.clean?.length ?? 0} label="clean" color="green" />
                        <StatusBadge count={result.withChanges?.length ?? 0} label="changed" color="yellow" />
                        <StatusBadge count={result.withUnpushed?.length ?? 0} label="unpushed" color="orange" />
                        <StatusBadge count={result.withUnpulled?.length ?? 0} label="unpulled" color="purple" />
                        <StatusBadge count={result.uninitialized?.length ?? 0} label="uninitialized" color="gray" />
                      </div>
                    ) : (
                      <span className="text-text-muted text-xs">Not scanned</span>
                    )}
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      scan([folder]);
                    }}
                    disabled={isFolderScanning}
                    className="px-2 py-0.5 rounded-sm bg-dark-borderStrong hover:bg-dark-elevated text-text-secondary hover:text-text-primary text-xs font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed flex-shrink-0"
                  >
                    Scan
                  </button>
                </button>

                {result && isExpanded && (
                  <div
                    className={`bg-dark-bg/50 border-t border-dark-border flex flex-col ${isLast ? 'flex-1 min-h-0' : ''}`}
                    onClick={(e) => e.stopPropagation()}
                  >
                    {/* Repo sections - scrollable */}
                    <div className={`px-2.5 py-2.5 space-y-3 ${isLast ? 'flex-1 overflow-auto' : ''}`}>
                      <RepoSection title="Uncommitted Changes" repos={result.withChanges || []} color="yellow" onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} onOpenInEditor={handleOpenInEditor} defaultTerminalName={defaultTerminal?.displayName} defaultEditorName={defaultEditor?.displayName} pullingRepos={pullingRepos} disablePull />
                      <RepoSection title="Unpushed Commits" repos={result.withUnpushed || []} color="orange" onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} onOpenInEditor={handleOpenInEditor} defaultTerminalName={defaultTerminal?.displayName} defaultEditorName={defaultEditor?.displayName} pullingRepos={pullingRepos} />
                      <RepoSection title="Unpulled Commits" repos={result.withUnpulled || []} color="purple" onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} onOpenInEditor={handleOpenInEditor} defaultTerminalName={defaultTerminal?.displayName} defaultEditorName={defaultEditor?.displayName} pullingRepos={pullingRepos} onPullAll={() => handlePullAllUnpulled(result.withUnpulled || [])} isPullingAll={isPullingAllUnpulled} />
                      <RepoSection title="Clean" repos={result.clean || []} color="green" muted onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} onOpenInEditor={handleOpenInEditor} onClean={handleClean} defaultTerminalName={defaultTerminal?.displayName} defaultEditorName={defaultEditor?.displayName} pullingRepos={pullingRepos} cleaningRepos={cleaningRepos} showCleanOption />
                      <RepoSection title="Uninitialized" repos={result.uninitialized || []} color="gray" muted onOpenInTerminal={handleOpenInTerminal} onOpenInEditor={handleOpenInEditor} defaultTerminalName={defaultTerminal?.displayName} defaultEditorName={defaultEditor?.displayName} />
                      <RepoSection title="Errors" repos={result.errors || []} color="red" showErrors onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} onOpenInEditor={handleOpenInEditor} defaultTerminalName={defaultTerminal?.displayName} defaultEditorName={defaultEditor?.displayName} pullingRepos={pullingRepos} disablePull />
                    </div>

                    {/* Execution Time - pinned at bottom */}
                    <div className="flex-shrink-0 text-text-muted text-xs border-t border-dark-borderSubtle px-2.5 py-2 opacity-70">
                      Completed in {(result.executionTime ?? 0).toFixed(2)}s
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
