import { useState, useRef } from 'react';
// @ts-ignore - Wails binding not properly typed
import { ScanFolder } from '../../wailsjs/go/main/App';
import { services } from '../../wailsjs/go/models';

interface MonitoredFolder {
  id: string;
  path: string;
  name: string;
}

type ScanResult = services.ScanResult;

interface ScanResultsProps {
  folders: MonitoredFolder[];
}

export default function ScanResults({ folders }: ScanResultsProps) {
  const [results, setResults] = useState<Record<string, ScanResult>>({});
  const [scanningFolders, setScanningFolders] = useState<Record<string, boolean>>({});
  const [isFullScanActive, setIsFullScanActive] = useState(false);
  const [error, setError] = useState('');
  const [expandedFolder, setExpandedFolder] = useState<string | null>(null);
  const scanVersionRef = useRef(0);

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
          const result = await ScanFolder(folder.path);
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

      if (isFullScan && foldersToScan.length === 1) {
        setExpandedFolder(foldersToScan[0].id);
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
    <div className="bg-dark-surface rounded-lg shadow-lg p-6 border border-dark-border">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold text-white">Repository Status</h2>
        <button
          onClick={() => scan(folders, true)}
          disabled={folders.length === 0}
          className="bg-green-600 hover:bg-green-700 text-white font-semibold py-2 px-4 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isFullScanActive ? 'Scanning...' : '🔍 Scan All'}
        </button>
      </div>

      {error && (
        <div className="bg-red-900 border border-red-700 text-red-100 px-4 py-3 rounded-lg mb-4">
          {error}
        </div>
      )}

      {folders.length === 0 ? (
        <p className="text-gray-400 text-center py-8">
          No folders configured. Add a folder to get started.
        </p>
      ) : (
        <div className="space-y-4">
          {folders.map((folder) => {
            const result = results[folder.id];
            const isExpanded = expandedFolder === folder.id;
            const isFolderScanning = scanningFolders[folder.id] ?? false;

            return (
              <div
                key={folder.id}
                className="bg-dark-bg rounded-lg border border-dark-border overflow-hidden hover:border-blue-500 transition-colors"
              >
                <button
                  onClick={() => setExpandedFolder(isExpanded ? null : folder.id)}
                  className="w-full flex justify-between items-center p-4 hover:bg-dark-surface transition-colors text-left"
                >
                  <div className="flex-1">
                    <h3 className="text-white font-semibold">{folder.name}</h3>
                    <p className="text-gray-400 text-sm">{folder.path}</p>
                  </div>
                  <div className="text-right mx-4">
                    {isFolderScanning ? (
                      <span className="text-gray-400 text-sm">Scanning...</span>
                    ) : result ? (
                      <div className="text-sm">
                        <span className="text-gray-300">
                          {result.totalRepositories} repos
                        </span>
                        <span className="mx-2 text-gray-500">•</span>
                        <span className="text-green-400">
                          {(result.clean?.length ?? 0)} clean
                        </span>
                        <span className="mx-2 text-gray-500">•</span>
                        <span className="text-yellow-400">
                          {(result.withChanges?.length ?? 0)} changes
                        </span>
                        <span className="mx-2 text-gray-500">•</span>
                        <span className="text-orange-400">
                          {(result.withUnpushed?.length ?? 0)} unpushed
                        </span>
                      </div>
                    ) : (
                      <span className="text-gray-400 text-sm">Not scanned</span>
                    )}
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      scan([folder]);
                    }}
                    disabled={isFolderScanning}
                    className="ml-2 px-3 py-1 rounded bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex-shrink-0"
                  >
                    🔍
                  </button>
                </button>

                {result && isExpanded && (
                  <div className="bg-dark-surface border-t border-dark-border p-4 space-y-6">
                    {/* Uncommitted Changes */}
                    {(result.withChanges?.length ?? 0) > 0 && (
                      <div>
                        <h4 className="text-yellow-400 font-semibold mb-3 flex items-center gap-2">
                          📝 Uncommitted Changes ({result.withChanges?.length ?? 0})
                        </h4>
                        <ul className="space-y-2">
                          {(result.withChanges || []).map((repo: services.RepoStatus, idx: number) => (
                            <li
                              key={idx}
                              className="text-gray-300 text-sm ml-4 py-1 border-l-2 border-yellow-500 pl-3 hover:text-white transition-colors"
                            >
                              {repo.path}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Unpushed Commits */}
                    {(result.withUnpushed?.length ?? 0) > 0 && (
                      <div>
                        <h4 className="text-orange-400 font-semibold mb-3 flex items-center gap-2">
                          ⬆ Unpushed Commits ({result.withUnpushed?.length ?? 0})
                        </h4>
                        <ul className="space-y-2">
                          {(result.withUnpushed || []).map((repo: services.RepoStatus, idx: number) => (
                            <li
                              key={idx}
                              className="text-gray-300 text-sm ml-4 py-1 border-l-2 border-orange-500 pl-3 hover:text-white transition-colors"
                            >
                              {repo.path}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Clean Repositories */}
                    {(result.clean?.length ?? 0) > 0 && (
                      <div>
                        <h4 className="text-green-400 font-semibold mb-3 flex items-center gap-2">
                          ✓ Clean ({result.clean?.length ?? 0})
                        </h4>
                        <ul className="space-y-1 max-h-40 overflow-y-auto">
                          {(result.clean || []).map((repo: services.RepoStatus, idx: number) => (
                            <li
                              key={idx}
                              className="text-gray-400 text-sm ml-4 py-1 border-l-2 border-green-500 pl-3 hover:text-green-300 transition-colors"
                            >
                              {repo.path}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Errors */}
                    {(result.errors?.length ?? 0) > 0 && (
                      <div>
                        <h4 className="text-red-400 font-semibold mb-3 flex items-center gap-2">
                          ⚠ Errors ({result.errors?.length ?? 0})
                        </h4>
                        <ul className="space-y-2">
                          {(result.errors || []).map((repo: services.RepoStatus, idx: number) => (
                            <li key={idx} className="text-red-300 text-sm ml-4 py-1">
                              <p className="border-l-2 border-red-500 pl-3">
                                {repo.path}
                              </p>
                              {repo.errorMessage && (
                                <p className="text-red-400 text-xs mt-1 ml-3">
                                  {repo.errorMessage}
                                </p>
                              )}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {/* Execution Time */}
                    <div className="text-gray-400 text-sm border-t border-dark-border pt-4">
                      ⏱ Scan completed in {(result.executionTime ?? 0).toFixed(2)}s
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
