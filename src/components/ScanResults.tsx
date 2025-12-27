import { useState, useRef, useEffect } from 'react';
import { api } from '../lib/api';
import { MonitoredFolder, ScanResult, RepoStatus, TerminalApp } from '../types';

// Three-dot menu icon
function DotsIcon() {
  return (
    <svg className="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
      <path d="M10 6a2 2 0 110-4 2 2 0 010 4zM10 12a2 2 0 110-4 2 2 0 010 4zM10 18a2 2 0 110-4 2 2 0 010 4z" />
    </svg>
  );
}

export interface ScanResultsState {
  results: Record<string, ScanResult>;
  expandedFolders: Set<string>;
}

interface ScanResultsProps {
  folders: MonitoredFolder[];
  scanState: ScanResultsState;
  onScanStateChange: React.Dispatch<React.SetStateAction<ScanResultsState>>;
  defaultTerminal: TerminalApp | null;
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
  onPull?: (repoPath: string) => void;
  onPullAll?: () => void;
  onOpenInTerminal?: (repoPath: string) => void;
  pullingRepos?: Set<string>;
  disablePull?: boolean;
  isPullingAll?: boolean;
  defaultTerminalName?: string;
}

function RepoSection({
  title,
  repos,
  color,
  muted = false,
  scrollable = false,
  showErrors = false,
  onPull,
  onPullAll,
  onOpenInTerminal,
  pullingRepos = new Set(),
  disablePull = false,
  isPullingAll = false,
  defaultTerminalName,
}: RepoSectionProps) {
  const [openMenuPath, setOpenMenuPath] = useState<string | null>(null);
  const [menuPosition, setMenuPosition] = useState<{ top: number; left: number } | null>(null);
  const [showSectionMenu, setShowSectionMenu] = useState(false);
  const [sectionMenuPosition, setSectionMenuPosition] = useState<{ top: number; left: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const sectionMenuRef = useRef<HTMLDivElement>(null);
  const sectionMenuButtonRef = useRef<HTMLButtonElement>(null);
  const buttonRefs = useRef<Map<string, HTMLButtonElement>>(new Map());

  // Close menu when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setOpenMenuPath(null);
      }
      if (sectionMenuRef.current && !sectionMenuRef.current.contains(event.target as Node) &&
          sectionMenuButtonRef.current && !sectionMenuButtonRef.current.contains(event.target as Node)) {
        setShowSectionMenu(false);
      }
    }
    if (openMenuPath || showSectionMenu) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [openMenuPath, showSectionMenu]);

  const openMenu = (repoPath: string) => {
    const button = buttonRefs.current.get(repoPath);
    if (button) {
      const rect = button.getBoundingClientRect();
      setMenuPosition({
        top: rect.bottom + 4,
        left: rect.right - 140, // 140px is min-width of menu
      });
    }
    setOpenMenuPath(repoPath);
  };

  const openSectionMenu = () => {
    if (sectionMenuButtonRef.current) {
      const rect = sectionMenuButtonRef.current.getBoundingClientRect();
      setSectionMenuPosition({
        top: rect.bottom + 4,
        left: rect.right - 160,
      });
    }
    setShowSectionMenu(true);
  };

  if (repos.length === 0) return null;

  const styles = colorStyles[color];

  return (
    <div>
      <div className="flex items-center justify-between mb-1.5">
        <h4 className={`${styles.text} text-xs font-medium uppercase tracking-wider opacity-90`}>
          {title} ({repos.length})
        </h4>
        {onPullAll && (
          <>
            <button
              ref={sectionMenuButtonRef}
              onClick={() => showSectionMenu ? setShowSectionMenu(false) : openSectionMenu()}
              className={`p-1 rounded hover:bg-dark-border transition-colors ${showSectionMenu ? 'bg-dark-border' : ''}`}
              disabled={isPullingAll}
            >
              {isPullingAll ? (
                <span className="w-3.5 h-3.5 block border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
              ) : (
                <DotsIcon />
              )}
            </button>
            {showSectionMenu && sectionMenuPosition && (
              <div
                ref={sectionMenuRef}
                className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-1 min-w-[160px]"
                style={{ top: sectionMenuPosition.top, left: sectionMenuPosition.left }}
              >
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setShowSectionMenu(false);
                    onPullAll();
                  }}
                  className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors"
                >
                  Fetch & Pull All ({repos.length})
                </button>
              </div>
            )}
          </>
        )}
      </div>
      <ul className={`space-y-px ${scrollable ? 'max-h-64 overflow-y-auto' : ''}`}>
        {repos.map((repo, idx) => {
          const isPulling = pullingRepos.has(repo.path);
          const isMenuOpen = openMenuPath === repo.path;

          return (
            <li key={idx} className={`relative group ${showErrors ? 'text-xs' : ''}`}>
              <div className={`flex items-center ${muted ? 'text-text-muted' : 'text-text-secondary'} text-xs py-0.5 pl-2 pr-1 border-l-2 ${muted ? styles.borderMuted : styles.border} hover:text-text-primary hover:bg-dark-borderSubtle transition-colors font-mono rounded-r-sm`}>
                <span className="flex-1 truncate">
                  {repo.path}
                  {repo.branch && (
                    <span className={`ml-1.5 ${muted ? 'text-accent-blue/60' : 'text-accent-blue/70'}`}>
                      ({repo.branch})
                    </span>
                  )}
                </span>
                {onPull && (
                  <>
                    <button
                      ref={(el) => {
                        if (el) buttonRefs.current.set(repo.path, el);
                      }}
                      onClick={(e) => {
                        e.stopPropagation();
                        if (isMenuOpen) {
                          setOpenMenuPath(null);
                        } else {
                          openMenu(repo.path);
                        }
                      }}
                      className={`p-1 rounded hover:bg-dark-border transition-colors ${isMenuOpen ? 'bg-dark-border' : 'opacity-0 group-hover:opacity-100'}`}
                      disabled={isPulling}
                    >
                      {isPulling ? (
                        <span className="w-3.5 h-3.5 block border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
                      ) : (
                        <DotsIcon />
                      )}
                    </button>
                    {isMenuOpen && menuPosition && (
                      <div
                        ref={menuRef}
                        className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-1 min-w-[140px]"
                        style={{ top: menuPosition.top, left: menuPosition.left }}
                      >
                        {onOpenInTerminal && defaultTerminalName && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setOpenMenuPath(null);
                              onOpenInTerminal(repo.path);
                            }}
                            className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors"
                          >
                            Open in {defaultTerminalName}
                          </button>
                        )}
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            setOpenMenuPath(null);
                            onPull(repo.path);
                          }}
                          disabled={disablePull}
                          className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                        >
                          Fetch & Pull
                        </button>
                      </div>
                    )}
                  </>
                )}
              </div>
              {showErrors && repo.errorMessage && (
                <p className="text-text-muted text-xs mt-0.5 pl-2 opacity-80">
                  {repo.errorMessage}
                </p>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

export default function ScanResults({ folders, scanState, onScanStateChange, defaultTerminal }: ScanResultsProps) {
  const { results, expandedFolders } = scanState;
  const [scanningFolders, setScanningFolders] = useState<Record<string, boolean>>({});
  const [isFullScanActive, setIsFullScanActive] = useState(false);
  const [error, setError] = useState('');
  const [pullingRepos, setPullingRepos] = useState<Set<string>>(new Set());
  const [isPullingAllUnpulled, setIsPullingAllUnpulled] = useState(false);
  const scanVersionRef = useRef(0);
  const hasInitialScanRef = useRef(false);

  // Auto-scan all folders on app startup
  useEffect(() => {
    if (folders.length > 0 && !hasInitialScanRef.current) {
      hasInitialScanRef.current = true;
      scan(folders, true);
    }
  }, [folders]);

  const handleOpenInTerminal = async (repoPath: string) => {
    if (!defaultTerminal) return;
    try {
      await api.openInTerminal(repoPath, defaultTerminal.id);
    } catch (err) {
      setError(`Failed to open terminal: ${err}`);
    }
  };

  const handlePull = async (repoPath: string) => {
    setPullingRepos(prev => new Set(prev).add(repoPath));
    try {
      await api.pullRepo(repoPath);
      // Re-scan all folders to update status
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

  const handlePullAllUnpulled = async (unpulledRepos: RepoStatus[]) => {
    if (unpulledRepos.length === 0) return;

    setIsPullingAllUnpulled(true);
    const repoPaths = unpulledRepos.map(r => r.path);
    setPullingRepos(prev => new Set([...prev, ...repoPaths]));

    try {
      // Pull all repos in parallel
      const pullPromises = repoPaths.map(path =>
        api.pullRepo(path).catch(err => ({ path, error: err }))
      );
      const results = await Promise.all(pullPromises);

      // Check for errors
      const errors = results.filter(r => r && typeof r === 'object' && 'error' in r);
      if (errors.length > 0) {
        setError(`Failed to pull ${errors.length} repo(s)`);
      }

      // Re-scan all folders to update status
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
                      <RepoSection title="Uncommitted Changes" repos={result.withChanges || []} color="yellow" onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} defaultTerminalName={defaultTerminal?.name} pullingRepos={pullingRepos} disablePull />
                      <RepoSection title="Unpushed Commits" repos={result.withUnpushed || []} color="orange" onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} defaultTerminalName={defaultTerminal?.name} pullingRepos={pullingRepos} />
                      <RepoSection title="Unpulled Commits" repos={result.withUnpulled || []} color="purple" onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} defaultTerminalName={defaultTerminal?.name} pullingRepos={pullingRepos} onPullAll={() => handlePullAllUnpulled(result.withUnpulled || [])} isPullingAll={isPullingAllUnpulled} />
                      <RepoSection title="Clean" repos={result.clean || []} color="green" muted onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} defaultTerminalName={defaultTerminal?.name} pullingRepos={pullingRepos} />
                      <RepoSection title="Errors" repos={result.errors || []} color="red" showErrors onPull={handlePull} onOpenInTerminal={handleOpenInTerminal} defaultTerminalName={defaultTerminal?.name} pullingRepos={pullingRepos} disablePull />
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
