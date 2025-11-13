import { useState } from 'react';
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
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState('');
  const [expandedFolder, setExpandedFolder] = useState<string | null>(null);

  console.log('[ScanResults] Rendering with folders:', folders, 'results:', results);

  const handleScanFolder = async (folder: MonitoredFolder) => {
    try {
      console.log('[ScanResults] Scanning folder:', folder.name, folder.path);
      setError('');
      setIsScanning(true);
      const result = await ScanFolder(folder.path);
      console.log('[ScanResults] Scan result:', result);
      setResults((prev) => ({
        ...prev,
        [folder.id]: result,
      }));
      setExpandedFolder(folder.id);
    } catch (err) {
      setError(`Failed to scan folder: ${folder.name}`);
      console.error(err);
    } finally {
      setIsScanning(false);
    }
  };

  const handleScanAll = async () => {
    try {
      setError('');
      setIsScanning(true);
      const newResults: Record<string, ScanResult> = {};

      for (const folder of folders) {
        const result = await ScanFolder(folder.path);
        newResults[folder.id] = result;
      }

      setResults(newResults);
    } catch (err) {
      setError('Failed to scan folders');
      console.error(err);
    } finally {
      setIsScanning(false);
    }
  };

  return (
    <div className="bg-dark-surface rounded-lg shadow-lg p-6 border border-dark-border">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold text-white">Repository Status</h2>
        <button
          onClick={handleScanAll}
          disabled={isScanning || folders.length === 0}
          className="bg-green-600 hover:bg-green-700 text-white font-semibold py-2 px-4 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isScanning ? 'Scanning...' : '🔍 Scan All'}
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

            return (
              <div
                key={folder.id}
                className="bg-dark-bg rounded-lg border border-dark-border overflow-hidden hover:border-blue-500 transition-colors"
              >
                <button
                  onClick={() => {
                    if (!isScanning) {
                      handleScanFolder(folder);
                    }
                  }}
                  disabled={isScanning}
                  className="w-full flex justify-between items-center p-4 hover:bg-dark-surface transition-colors disabled:cursor-not-allowed"
                >
                  <div className="flex-1 text-left">
                    <h3 className="text-white font-semibold">{folder.name}</h3>
                    <p className="text-gray-400 text-sm">{folder.path}</p>
                  </div>
                  <div className="text-right ml-4">
                    {result ? (
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
                      <span className="text-gray-400 text-sm">
                        {isScanning ? 'Scanning...' : 'Not scanned'}
                      </span>
                    )}
                  </div>
                  <span className="ml-4 text-gray-400">
                    {isExpanded ? '▼' : '▶'}
                  </span>
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
