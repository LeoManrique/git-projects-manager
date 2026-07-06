import { useState, useEffect, useCallback, useMemo } from 'react';
import { FolderManager } from './components/folders';
import { AllFoldersOverview, FolderDetail, RepoActionHandlers } from './components/scan';
import DefaultAppsSettings from './components/settings/DefaultAppsSettings';
import GitCleanSettings from './components/settings/GitCleanSettings';
import AccountSettings from './components/settings/AccountSettings';
import { KanbanBoard } from './components/kanban';
import { Sidebar, Selection } from './components/navigation';
import { api } from './lib/api';
import { MonitoredFolder, TerminalApp, EditorApp } from './types';
import { useScanner } from './hooks/useScanner';
import { FolderIcon, AppsIcon, CloseIcon, CleanIcon, UserIcon, RefreshIcon, SearchIcon } from './components/icons';
import './App.css';

type SettingsCategory = 'folders' | 'apps' | 'gitclean' | 'account';

interface SettingsModalProps {
  isOpen: boolean;
  initialCategory: SettingsCategory;
  onClose: () => void;
  folders: MonitoredFolder[];
  onRefreshFolders: () => Promise<void>;
  onSettingsChange: () => void;
}

function SettingsModal({ isOpen, initialCategory, onClose, folders, onRefreshFolders, onSettingsChange }: SettingsModalProps) {
  const [activeCategory, setActiveCategory] = useState<SettingsCategory>(initialCategory);

  // Re-open on the category the caller asked for (e.g. sidebar "Add Folder").
  useEffect(() => {
    if (isOpen) setActiveCategory(initialCategory);
  }, [isOpen, initialCategory]);

  if (!isOpen) return null;

  const categories: { id: SettingsCategory; label: string; icon: React.ReactNode }[] = [
    { id: 'folders', label: 'Monitored Folders', icon: <FolderIcon /> },
    { id: 'apps', label: 'Default Apps', icon: <AppsIcon /> },
    { id: 'gitclean', label: 'Git Clean', icon: <CleanIcon /> },
    { id: 'account', label: 'Account', icon: <UserIcon /> },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-[2px]"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative bg-dark-bg border border-dark-border rounded-lg shadow-2xl w-[680px] h-[480px] flex overflow-hidden">

        {/* Left Sidebar - Categories */}
        <div className="w-[180px] bg-dark-surface/50 border-r border-dark-border flex flex-col">
          {/* Sidebar Header */}
          <div className="h-12 flex items-center px-4 border-b border-dark-border/50">
            <span className="text-xs font-semibold text-text-secondary uppercase tracking-wider">Settings</span>
          </div>

          {/* Category List */}
          <nav className="flex-1 p-2 space-y-0.5">
            {categories.map((category) => (
              <button
                key={category.id}
                onClick={() => setActiveCategory(category.id)}
                className={`w-full flex items-center gap-2.5 px-2.5 py-2 rounded text-left transition-all ${
                  activeCategory === category.id
                    ? 'bg-accent-blue/15 text-accent-blue'
                    : 'text-text-secondary hover:text-text-primary hover:bg-dark-borderSubtle'
                }`}
              >
                <span className={activeCategory === category.id ? 'text-accent-blue' : 'text-text-muted'}>
                  {category.icon}
                </span>
                <span className="text-xs font-medium">{category.label}</span>
              </button>
            ))}
          </nav>

          {/* Sidebar Footer */}
          <div className="p-3 border-t border-dark-border/50">
            <p className="text-[10px] text-text-muted leading-tight">
              Git Projects Manager v2.2
            </p>
          </div>
        </div>

        {/* Right Content Area */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Content Header */}
          <div className="h-12 flex items-center justify-between px-4 border-b border-dark-border/50 bg-dark-surface/30">
            <h2 className="text-sm font-medium text-text-primary">
              {categories.find(c => c.id === activeCategory)?.label}
            </h2>
            <button
              onClick={onClose}
              className="p-1.5 rounded hover:bg-dark-borderSubtle transition-colors text-text-muted hover:text-text-primary"
              title="Close"
            >
              <CloseIcon />
            </button>
          </div>

          {/* Content Body */}
          <div className="flex-1 overflow-auto p-4">
            {activeCategory === 'folders' && (
              <FolderManager folders={folders} onRefresh={onRefreshFolders} />
            )}
            {activeCategory === 'apps' && (
              <DefaultAppsSettings onSettingsChange={onSettingsChange} />
            )}
            {activeCategory === 'gitclean' && (
              <GitCleanSettings onSettingsChange={onSettingsChange} />
            )}
            {activeCategory === 'account' && (
              <AccountSettings />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function App() {
  const [folders, setFolders] = useState<MonitoredFolder[]>([]);
  const [selection, setSelection] = useState<Selection>({ view: 'all' });
  const [searchQuery, setSearchQuery] = useState('');
  const [settings, setSettings] = useState<{ open: boolean; category: SettingsCategory }>({
    open: false,
    category: 'folders',
  });
  const [defaultTerminal, setDefaultTerminal] = useState<TerminalApp | null>(null);
  const [defaultEditor, setDefaultEditor] = useState<EditorApp | null>(null);

  const scanner = useScanner(folders);

  const loadFolders = useCallback(async () => {
    try {
      const loaded = await api.getMonitoredFolders();
      setFolders(loaded || []);
    } catch (err) {
      console.error('Failed to load folders:', err);
    }
  }, []);

  const loadAppSettings = useCallback(async () => {
    try {
      const [appSettings, terminals, editors] = await Promise.all([
        api.getAppSettings(),
        api.getAvailableTerminals(),
        api.getAvailableEditors(),
      ]);
      setDefaultTerminal(terminals.find(t => t.id === appSettings.defaultTerminal) ?? null);
      setDefaultEditor(editors.find(e => e.id === appSettings.defaultEditor) ?? null);
    } catch (err) {
      console.error('Failed to load app settings:', err);
    }
  }, []);

  useEffect(() => {
    loadFolders();
    loadAppSettings();
  }, [loadFolders, loadAppSettings]);

  // A deleted folder can't stay selected — fall back to the overview.
  useEffect(() => {
    if (selection.view === 'folder' && !folders.some(f => f.id === selection.folderId)) {
      setSelection({ view: 'all' });
    }
  }, [folders, selection]);

  const { setError } = scanner;
  const repoHandlers: RepoActionHandlers = useMemo(() => ({
    onPull: (path) => void scanner.pull(path),
    onClean: (path) => void scanner.clean(path),
    onOpenInTerminal: async (path) => {
      if (!defaultTerminal) return;
      try {
        await api.openInTerminal(path, defaultTerminal.id);
      } catch (err) {
        setError(`Failed to open terminal: ${err}`);
      }
    },
    onOpenInEditor: async (path) => {
      if (!defaultEditor) return;
      try {
        await api.openInEditor(path, defaultEditor.id);
      } catch (err) {
        setError(`Failed to open editor: ${err}`);
      }
    },
    onOpenInLmsGithub: async (path) => {
      try {
        await api.openInLmsGithub(path);
      } catch (err) {
        setError(`Failed to open LMS Github: ${err}`);
      }
    },
    defaultTerminalName: defaultTerminal?.displayName,
    defaultEditorName: defaultEditor?.displayName,
    pullingRepos: scanner.pullingRepos,
    cleaningRepos: scanner.cleaningRepos,
  }), [scanner.pull, scanner.clean, scanner.pullingRepos, scanner.cleaningRepos, setError, defaultTerminal, defaultEditor]);

  const openSettings = (category: SettingsCategory) => setSettings({ open: true, category });

  const selectedFolder =
    selection.view === 'folder' ? folders.find(f => f.id === selection.folderId) : undefined;
  const title =
    selection.view === 'kanban' ? 'Kanban' : selectedFolder ? selectedFolder.name : 'All Folders';

  return (
    <div className="h-screen flex bg-dark-bg text-text-primary overflow-hidden">
      <Sidebar
        folders={folders}
        results={scanner.results}
        scanningFolders={scanner.scanningFolders}
        selection={selection}
        onSelect={setSelection}
        onAddFolder={() => openSettings('folders')}
        onOpenSettings={() => openSettings('apps')}
      />

      <div className="flex-1 flex flex-col min-w-0">
        {/* Content header: title + search + Scan All */}
        <header className="flex-shrink-0 flex items-center gap-3 px-4 py-2 border-b border-dark-border bg-dark-surface/30">
          <div className="flex-1 min-w-0">
            <h1 className="text-base font-semibold text-text-primary truncate leading-tight">{title}</h1>
            {selectedFolder && (
              <p className="text-[11px] leading-4 font-mono text-text-muted truncate">{selectedFolder.path}</p>
            )}
          </div>
          {selection.view !== 'kanban' && (
            <>
              <div className="relative w-60 max-w-[40%]">
                <span className="absolute left-2 top-1/2 -translate-y-1/2 text-text-muted pointer-events-none">
                  <SearchIcon />
                </span>
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search repositories"
                  className="w-full pl-7 pr-2 py-1 bg-dark-bg border border-dark-border rounded-md text-text-primary text-xs placeholder-text-muted focus:outline-none focus:border-accent-blue/50"
                />
              </div>
              <button
                onClick={scanner.scanAll}
                disabled={folders.length === 0}
                className="flex-shrink-0 flex items-center gap-1.5 bg-accent-blue hover:bg-accent-blueHover text-white text-xs font-medium py-1.5 px-3 rounded-md transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              >
                {scanner.isFullScanning ? (
                  <>
                    <span className="w-3 h-3 border-2 border-white/70 border-t-transparent rounded-full animate-spin" />
                    Scanning…
                  </>
                ) : (
                  <>
                    <RefreshIcon className="w-3 h-3" />
                    Scan All
                  </>
                )}
              </button>
            </>
          )}
        </header>

        {/* Shared error surface (FRONTEND.md §5.6) */}
        {selection.view !== 'kanban' && scanner.error && (
          <div className="flex-shrink-0 mx-3 mt-2 bg-accent-red/10 border border-accent-red/20 text-accent-red px-2.5 py-1.5 rounded text-xs">
            {scanner.error}
          </div>
        )}

        {/* Both views stay mounted so switches don't refetch. */}
        <main className="flex-1 min-h-0">
          <div className={selection.view !== 'kanban' ? 'h-full' : 'hidden'}>
            {selectedFolder ? (
              <FolderDetail
                folder={selectedFolder}
                scanner={scanner}
                searchQuery={searchQuery}
                handlers={repoHandlers}
              />
            ) : (
              <AllFoldersOverview
                folders={folders}
                scanner={scanner}
                searchQuery={searchQuery}
                handlers={repoHandlers}
                onOpenFolder={(folderId) => setSelection({ view: 'folder', folderId })}
                onAddFolder={() => openSettings('folders')}
              />
            )}
          </div>
          <div className={selection.view === 'kanban' ? 'h-full' : 'hidden'}>
            <KanbanBoard />
          </div>
        </main>
      </div>

      <SettingsModal
        isOpen={settings.open}
        initialCategory={settings.category}
        onClose={() => setSettings(s => ({ ...s, open: false }))}
        folders={folders}
        onRefreshFolders={loadFolders}
        onSettingsChange={loadAppSettings}
      />
    </div>
  );
}

export default App;
