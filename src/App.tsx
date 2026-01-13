import { useState, useEffect, useCallback } from 'react';
import { FolderManager } from './components/folders';
import ScanResults, { ScanResultsState } from './components/ScanResults';
import DefaultAppsSettings from './components/settings/DefaultAppsSettings';
import { api } from './lib/api';
import { MonitoredFolder, TerminalApp, EditorApp } from './types';
import { useContextMenu } from './hooks';
import { DotsIcon, FolderIcon, AppsIcon, CloseIcon } from './components/icons';
import './App.css';

type SettingsCategory = 'folders' | 'apps';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  folders: MonitoredFolder[];
  onRefreshFolders: () => Promise<void>;
  onSettingsChange: () => void;
}

function SettingsModal({ isOpen, onClose, folders, onRefreshFolders, onSettingsChange }: SettingsModalProps) {
  const [activeCategory, setActiveCategory] = useState<SettingsCategory>('folders');

  if (!isOpen) return null;

  const categories: { id: SettingsCategory; label: string; icon: React.ReactNode }[] = [
    { id: 'folders', label: 'Monitored Folders', icon: <FolderIcon /> },
    { id: 'apps', label: 'Default Apps', icon: <AppsIcon /> },
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
              Git Projects Manager v2.0
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
          </div>
        </div>
      </div>
    </div>
  );
}

function App() {
  const [folders, setFolders] = useState<MonitoredFolder[]>([]);
  const [scanState, setScanState] = useState<ScanResultsState>({
    results: {},
    expandedFolders: new Set(),
  });
  const [showSettings, setShowSettings] = useState(false);
  const [defaultTerminal, setDefaultTerminal] = useState<TerminalApp | null>(null);
  const [defaultEditor, setDefaultEditor] = useState<EditorApp | null>(null);

  const headerMenu = useContextMenu({ menuWidth: 120 });

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
      const [settings, terminals, editors] = await Promise.all([
        api.getAppSettings(),
        api.getAvailableTerminals(),
        api.getAvailableEditors(),
      ]);
      if (settings.defaultTerminal) {
        const terminal = terminals.find(t => t.id === settings.defaultTerminal);
        setDefaultTerminal(terminal ?? null);
      } else {
        setDefaultTerminal(null);
      }
      if (settings.defaultEditor) {
        const editor = editors.find(e => e.id === settings.defaultEditor);
        setDefaultEditor(editor ?? null);
      } else {
        setDefaultEditor(null);
      }
    } catch (err) {
      console.error('Failed to load app settings:', err);
    }
  }, []);

  useEffect(() => {
    loadFolders();
    loadAppSettings();
  }, [loadFolders, loadAppSettings]);

  return (
    <div className="h-screen flex flex-col bg-dark-bg text-text-primary overflow-hidden">

      {/* Custom Title Bar */}
      <header
        data-tauri-drag-region
        className="flex-shrink-0 h-11 flex items-center justify-between px-3 bg-dark-surface/50 border-b border-dark-border"
        style={{ paddingLeft: '80px' }}
      >
        <span
          data-tauri-drag-region
          className="text-xs font-medium text-text-secondary select-none pointer-events-none"
        >
          Git Projects Manager
        </span>
        <button
          ref={headerMenu.buttonRef}
          onClick={headerMenu.toggle}
          className={`p-1.5 rounded hover:bg-dark-border transition-colors text-text-secondary hover:text-text-primary ${headerMenu.isOpen ? 'bg-dark-border' : ''}`}
        >
          <DotsIcon />
        </button>
      </header>

      {/* Header Menu Dropdown */}
      {headerMenu.isOpen && headerMenu.position && (
        <div
          ref={headerMenu.menuRef}
          className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-1 min-w-[120px]"
          style={{ top: headerMenu.position.top, left: headerMenu.position.left }}
        >
          <button
            onClick={() => {
              headerMenu.close();
              setShowSettings(true);
            }}
            className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors"
          >
            Settings
          </button>
        </div>
      )}

      {/* Main Content */}
      <main className="flex-1 overflow-hidden">
        <ScanResults
          folders={folders}
          scanState={scanState}
          onScanStateChange={setScanState}
          defaultTerminal={defaultTerminal}
          defaultEditor={defaultEditor}
        />
      </main>

      {/* Settings Modal */}
      <SettingsModal
        isOpen={showSettings}
        onClose={() => setShowSettings(false)}
        folders={folders}
        onRefreshFolders={loadFolders}
        onSettingsChange={loadAppSettings}
      />
    </div>
  );
}

export default App;
