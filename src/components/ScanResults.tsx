import { useState, useRef, useEffect } from 'react';
import { api } from '../lib/api';
import { MonitoredFolder, ScanResult, RepoStatus } from '../types';

export interface ScanResultsState {
  results: Record<string, ScanResult>;
  expandedFolders: Set<string>;
}

interface ScanResultsProps {
  folders: MonitoredFolder[];
  scanState: ScanResultsState;
  onScanStateChange: React.Dispatch<React.SetStateAction<ScanResultsState>>;
}

// Color variant mappings for Tailwind (must be explicit for JIT compiler)
type ColorVariant = 'green' | 'yellow' | 'orange' | 'purple' | 'red';

const colorStyles: Record<ColorVariant, { badge: string; text: string; border: string; borderMuted: string }> = {
  green: {
    badge: 'bg-accent-green/10 text-accent-green',
    text: 'text-accent-green',
    border: 'border-accent-green/40',
    borderMuted: 'border-accent-green/30',
  },
  yellow: {
    badge: 'bg-accent-yellow/10 text-accent-yellow',
    text: 'text-accent-yellow',
    border: 'border-accent-yellow/40',
    borderMuted: 'border-accent-yellow/30',
  },
  orange: {
    badge: 'bg-accent-orange/10 text-accent-orange',
    text: 'text-accent-orange',
    border: 'border-accent-orange/40',
    borderMuted: 'border-accent-orange/30',
  },
  purple: {
    badge: 'bg-purple-500/10 text-purple-400',
    text: 'text-purple-400',
    border: 'border-purple-400/40',
    borderMuted: 'border-purple-400/30',
  },
  red: {
    badge: 'bg-accent-red/10 text-accent-red',
    text: 'text-accent-red',
    border: 'border-accent-red/40',
    borderMuted: 'border-accent-red/30',
  },
};

// Status badge component for the summary row
function StatusBadge({ count, label, color }: { count: number; label: string; color: ColorVariant }) {
  return (
    <span className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded-sm ${colorStyles[color].badge}`}>
      <span className="w-1.5 h-1.5 rounded-full bg-current" />
      {count} {label}
    </span>
  );
}

// Repository list section component
interface RepoSectionProps {
  title: string;
  repos: RepoStatus[];
  color: ColorVariant;
  muted?: boolean;
  scrollable?: boolean;
  showErrors?: boolean;
}

function RepoSection({ title, repos, color, muted = false, scrollable = false, showErrors = false }: RepoSectionProps) {
  if (repos.length === 0) return null;

  const styles = colorStyles[color];

  return (
    <div>
      <h4 className={`${styles.text} text-xs font-medium mb-1.5 uppercase tracking-wider opacity-90`}>
        {title} ({repos.length})
      </h4>
      <ul className={`space-y-px ${scrollable ? 'max-h-64 overflow-y-auto' : ''}`}>
        {repos.map((repo, idx) => (
          <li key={idx} className={showErrors ? 'text-xs' : undefined}>
            <p className={`${muted ? 'text-text-muted' : 'text-text-secondary'} text-xs py-0.5 pl-2 border-l-2 ${muted ? styles.borderMuted : styles.border} hover:text-text-primary hover:bg-dark-borderSubtle transition-colors font-mono rounded-r-sm`}>
              {repo.path}
              {repo.branch && (
                <span className={`ml-1.5 ${muted ? 'text-accent-blue/60' : 'text-accent-blue/70'}`}>
                  ({repo.branch})
                </span>
              )}
            </p>
            {showErrors && repo.errorMessage && (
              <p className="text-text-muted text-xs mt-0.5 pl-2 opacity-80">
                {repo.errorMessage}
              </p>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

export default function ScanResults({ folders, scanState, onScanStateChange }: ScanResultsProps) {
  const { results, expandedFolders } = scanState;
  const [scanningFolders, setScanningFolders] = useState<Record<string, boolean>>({});
  const [isFullScanActive, setIsFullScanActive] = useState(false);
  const [error, setError] = useState('');
  const scanVersionRef = useRef(0);
  const hasInitialScanRef = useRef(false);

  // Auto-scan all folders on app startup
  useEffect(() => {
    if (folders.length > 0 && !hasInitialScanRef.current) {
      hasInitialScanRef.current = true;
      scan(folders, true);
    }
  }, [folders]);

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
      setScanningFolders(prev => {
        const newState = { ...prev };
        for (const folder of foldersToScan) {
          newState[folder.id] = true;
        }
        return newState;
      });

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
      const newResults: Record<string, ScanResult> = {};
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

      // Merge new results with existing (functional update handles stale closure)
      onScanStateChange(prev => ({
        ...prev,
        results: { ...prev.results, ...newResults }
      }));

      if (isFullScan && foldersToScan.length === 1 && !expandedFolders.has(foldersToScan[0].id)) {
        toggleExpandedFolder(foldersToScan[0].id);
      }

      // Clear scanning state only for the folders we just scanned
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

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex-shrink-0 flex justify-between items-center mb-2">
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
        <div className="flex-shrink-0 bg-accent-red/10 border border-accent-red/20 text-accent-red px-2.5 py-1.5 rounded text-xs mb-2">
          {error}
        </div>
      )}

      {folders.length === 0 ? (
        <p className="text-text-muted text-xs text-center py-6">
          No folders configured. Add a folder to get started.
        </p>
      ) : (
        <div className="flex-1 overflow-auto space-y-1.5">
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
                  <div className="bg-dark-bg/50 border-t border-dark-border px-2.5 py-2.5 space-y-3">
                    <RepoSection title="Uncommitted Changes" repos={result.withChanges || []} color="yellow" />
                    <RepoSection title="Unpushed Commits" repos={result.withUnpushed || []} color="orange" />
                    <RepoSection title="Unpulled Commits" repos={result.withUnpulled || []} color="purple" />
                    <RepoSection title="Clean" repos={result.clean || []} color="green" muted scrollable />
                    <RepoSection title="Errors" repos={result.errors || []} color="red" showErrors />

                    {/* Execution Time */}
                    <div className="text-text-muted text-xs border-t border-dark-borderSubtle pt-2 opacity-70">
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
