import { useState, useRef } from 'react';
import { api } from '../lib/api';
import { MonitoredFolder, ScanResult, RepoStatus } from '../types';

export interface ScanResultsState {
  results: Record<string, ScanResult>;
  expandedFolders: Set<string>;
}

interface ScanResultsProps {
  folders: MonitoredFolder[];
  scanState: ScanResultsState;
  onScanStateChange: (state: ScanResultsState) => void;
}

export default function ScanResults({ folders, scanState, onScanStateChange }: ScanResultsProps) {
  const { results, expandedFolders } = scanState;
  const [scanningFolders, setScanningFolders] = useState<Record<string, boolean>>({});
  const [isFullScanActive, setIsFullScanActive] = useState(false);
  const [error, setError] = useState('');
  const scanVersionRef = useRef(0);

  const setResults = (newResults: Record<string, ScanResult>) => {
    onScanStateChange({ ...scanState, results: newResults });
  };

  const toggleExpandedFolder = (folderId: string) => {
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(folderId)) {
      newExpanded.delete(folderId);
    } else {
      newExpanded.add(folderId);
    }
    onScanStateChange({ ...scanState, expandedFolders: newExpanded });
  };

  const scan = async (foldersToScan: MonitoredFolder[], isFullScan: boolean = false) => {
    // Increment version for full scans to cancel previous scans
    let currentVersion = scanVersionRef.current;
    if (isFullScan) {
      currentVersion = ++scanVersionRef.current;
      setIsFullScanActive(true);
    }

    try {
      setError('');

      // Mark folders as scanning
      const scanningState: Record<string, boolean> = { ...scanningFolders };
      for (const folder of foldersToScan) {
        scanningState[folder.id] = true;
      }
      setScanningFolders(scanningState);

      // Scan folders in parallel
      const scanPromises = foldersToScan.map(async (folder) => {
        try {
          const result = await api.scanFolder(folder.path);
          return { id: folder.id, result, error: null };
        } catch (err) {
          return { id: folder.id, result: null, error: err };
        }
      });

      const scanResults = await Promise.allSettled(scanPromises);

      // Only update if this version is still current (for Scan All cancellation)
      if (currentVersion !== scanVersionRef.current) {
        setScanningFolders({});
        if (isFullScan) {
          setIsFullScanActive(false);
        }
        return;
      }

      // Update results
      const newResults: Record<string, ScanResult> = { ...results };
      const updatedFolderIds = new Set<string>();

      for (const result of scanResults) {
        if (result.status === 'fulfilled') {
          const { id, result: scanResult } = result.value;
          updatedFolderIds.add(id);
          if (scanResult) {
            newResults[id] = scanResult;
          }
        }
      }

      setResults(newResults);

      if (isFullScan && foldersToScan.length === 1 && !expandedFolders.has(foldersToScan[0].id)) {
        toggleExpandedFolder(foldersToScan[0].id);
      }

      // Clear scanning state only for the folders we just scanned
      const newScanningState = { ...scanningFolders };
      for (const folderId of updatedFolderIds) {
        delete newScanningState[folderId];
      }
      setScanningFolders(newScanningState);

      if (isFullScan) {
        setIsFullScanActive(false);
      }
    } catch (err) {
      setError('Failed to scan folder(s)');
      console.error(err);
      const newScanningState = { ...scanningFolders };
      for (const folder of foldersToScan) {
        delete newScanningState[folder.id];
      }
      setScanningFolders(newScanningState);
      if (isFullScan) {
        setIsFullScanActive(false);
      }
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex-shrink-0 flex justify-between items-center mb-3">
        <h2 className="text-base font-semibold text-text-primary">Repository Status</h2>
        <button
          onClick={() => scan(folders, true)}
          disabled={folders.length === 0}
          className="bg-accent-green hover:brightness-110 text-dark-bg text-xs font-medium py-1.5 px-3 rounded transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isFullScanActive ? 'Scanning...' : 'Scan All'}
        </button>
      </div>

      {error && (
        <div className="flex-shrink-0 bg-accent-red/10 border border-accent-red/30 text-accent-red px-3 py-2 rounded text-xs mb-3">
          {error}
        </div>
      )}

      {folders.length === 0 ? (
        <p className="text-text-muted text-sm text-center py-8">
          No folders configured. Add a folder to get started.
        </p>
      ) : (
        <div className="flex-1 overflow-auto space-y-2">
          {folders.map((folder) => {
            const result = results[folder.id];
            const isExpanded = expandedFolders.has(folder.id);
            const isFolderScanning = scanningFolders[folder.id] ?? false;

            return (
              <div
                key={folder.id}
                className="bg-dark-surface rounded border border-dark-border overflow-hidden"
              >
                <button
                  onClick={() => toggleExpandedFolder(folder.id)}
                  className="w-full flex justify-between items-center px-3 py-2.5 hover:bg-dark-elevated transition-colors text-left"
                >
                  <div className="flex-1 min-w-0">
                    <h3 className="text-text-primary text-sm font-medium truncate">{folder.name}</h3>
                    <p className="text-text-muted text-xs truncate font-mono">{folder.path}</p>
                  </div>
                  <div className="text-right mx-3 flex-shrink-0">
                    {isFolderScanning ? (
                      <span className="text-text-muted text-xs">Scanning...</span>
                    ) : result ? (
                      <div className="text-xs flex items-center gap-1.5">
                        <span className="text-text-secondary mr-1">
                          {result.totalRepositories} repos
                        </span>
                        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-accent-green/15 text-accent-green">
                          <span className="w-1.5 h-1.5 rounded-full bg-current" />
                          {(result.clean?.length ?? 0)} clean
                        </span>
                        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-accent-yellow/15 text-accent-yellow">
                          <span className="w-1.5 h-1.5 rounded-full bg-current" />
                          {(result.withChanges?.length ?? 0)} changed
                        </span>
                        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-accent-orange/15 text-accent-orange">
                          <span className="w-1.5 h-1.5 rounded-full bg-current" />
                          {(result.withUnpushed?.length ?? 0)} unpushed
                        </span>
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
                    className="px-2 py-1 rounded bg-accent-blue/20 hover:bg-accent-blue/30 text-accent-blue text-xs font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex-shrink-0"
                  >
                    Scan
                  </button>
                </button>

                {result && isExpanded && (
                  <div className="bg-dark-bg border-t border-dark-border px-3 py-3 space-y-4">
                    {/* Uncommitted Changes */}
                    {(result.withChanges?.length ?? 0) > 0 && (
                      <div>
                        <h4 className="text-accent-yellow text-xs font-medium mb-2">
                          Uncommitted Changes ({result.withChanges?.length ?? 0})
                        </h4>
                        <ul className="space-y-0.5">
                          {(result.withChanges || []).map((repo: RepoStatus, idx: number) => (
                            <li
                              key={idx}
                              className="text-text-secondary text-xs py-0.5 pl-2 border-l border-accent-yellow/50 hover:text-text-primary transition-colors font-mono"
                            >
                              {repo.path}
                              {repo.branch && (
                                <span className="ml-1.5 text-[#58a6ff]/80 font-light tracking-wide">
                                  ({repo.branch})
                                </span>
                              )}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Unpushed Commits */}
                    {(result.withUnpushed?.length ?? 0) > 0 && (
                      <div>
                        <h4 className="text-accent-orange text-xs font-medium mb-2">
                          Unpushed Commits ({result.withUnpushed?.length ?? 0})
                        </h4>
                        <ul className="space-y-0.5">
                          {(result.withUnpushed || []).map((repo: RepoStatus, idx: number) => (
                            <li
                              key={idx}
                              className="text-text-secondary text-xs py-0.5 pl-2 border-l border-accent-orange/50 hover:text-text-primary transition-colors font-mono"
                            >
                              {repo.path}
                              {repo.branch && (
                                <span className="ml-1.5 text-[#58a6ff]/80 font-light tracking-wide">
                                  ({repo.branch})
                                </span>
                              )}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Clean Repositories */}
                    {(result.clean?.length ?? 0) > 0 && (
                      <div>
                        <h4 className="text-accent-green text-xs font-medium mb-2">
                          Clean ({result.clean?.length ?? 0})
                        </h4>
                        <ul className="space-y-0.5 max-h-32 overflow-y-auto">
                          {(result.clean || []).map((repo: RepoStatus, idx: number) => (
                            <li
                              key={idx}
                              className="text-text-muted text-xs py-0.5 pl-2 border-l border-accent-green/50 hover:text-text-secondary transition-colors font-mono"
                            >
                              {repo.path}
                              {repo.branch && (
                                <span className="ml-1.5 text-[#58a6ff]/80 font-light tracking-wide">
                                  ({repo.branch})
                                </span>
                              )}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Errors */}
                    {(result.errors?.length ?? 0) > 0 && (
                      <div>
                        <h4 className="text-accent-red text-xs font-medium mb-2">
                          Errors ({result.errors?.length ?? 0})
                        </h4>
                        <ul className="space-y-1">
                          {(result.errors || []).map((repo: RepoStatus, idx: number) => (
                            <li key={idx} className="text-xs">
                              <p className="text-text-secondary pl-2 border-l border-accent-red/50 font-mono">
                                {repo.path}
                                {repo.branch && (
                                  <span className="ml-1.5 text-[#58a6ff]/80 font-light tracking-wide">
                                    ({repo.branch})
                                  </span>
                                )}
                              </p>
                              {repo.errorMessage && (
                                <p className="text-text-muted text-xs mt-0.5 pl-2">
                                  {repo.errorMessage}
                                </p>
                              )}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Execution Time */}
                    <div className="text-text-muted text-xs border-t border-dark-border pt-2">
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
