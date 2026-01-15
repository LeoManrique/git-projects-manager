import { useState, useRef, useEffect } from 'react';
import { RepoStatus } from '../../types';
import { colorStyles, ColorVariant } from './colorStyles';
import { DotsIcon } from '../icons';

export interface RepoSectionProps {
  title: string;
  repos: RepoStatus[];
  color: ColorVariant;
  muted?: boolean;
  scrollable?: boolean;
  showErrors?: boolean;
  onPull?: (repoPath: string) => void;
  onPullAll?: () => void;
  onOpenInTerminal?: (repoPath: string) => void;
  onOpenInEditor?: (repoPath: string) => void;
  onClean?: (repoPath: string) => void;
  pullingRepos?: Set<string>;
  cleaningRepos?: Set<string>;
  disablePull?: boolean;
  isPullingAll?: boolean;
  defaultTerminalName?: string;
  defaultEditorName?: string;
  showEditorOption?: boolean;
  showTerminalOption?: boolean;
  showPullOption?: boolean;
  showCleanOption?: boolean;
}

export function RepoSection({
  title,
  repos,
  color,
  muted = false,
  scrollable = false,
  showErrors = false,
  onPull,
  onPullAll,
  onOpenInTerminal,
  onOpenInEditor,
  onClean,
  pullingRepos = new Set(),
  cleaningRepos = new Set(),
  disablePull = false,
  isPullingAll = false,
  defaultTerminalName,
  defaultEditorName,
  showEditorOption = true,
  showTerminalOption = true,
  showPullOption = true,
  showCleanOption = false,
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
        left: rect.right - 140,
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
          const isCleaning = cleaningRepos.has(repo.path);
          const isOperating = isPulling || isCleaning;
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
                {((showEditorOption && onOpenInEditor) || (showTerminalOption && onOpenInTerminal) || (showPullOption && onPull) || (showCleanOption && onClean)) && (
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
                      disabled={isOperating}
                    >
                      {isOperating ? (
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
                        {showEditorOption && onOpenInEditor && defaultEditorName && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setOpenMenuPath(null);
                              onOpenInEditor(repo.path);
                            }}
                            className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors"
                          >
                            Open in {defaultEditorName}
                          </button>
                        )}
                        {showTerminalOption && onOpenInTerminal && defaultTerminalName && (
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
                        {showPullOption && onPull && (
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
                        )}
                        {showCleanOption && onClean && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setOpenMenuPath(null);
                              onClean(repo.path);
                            }}
                            className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors"
                          >
                            Clean Ignored Files
                          </button>
                        )}
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
