import { useState, useEffect, useCallback, useRef } from 'react';
import FolderManager from './components/FolderManager';
import ScanResults, { ScanResultsState } from './components/ScanResults';
import DefaultAppsSettings from './components/settings/DefaultAppsSettings';
import { api } from './lib/api';
import { MonitoredFolder, TerminalApp } from './types';
import './App.css';

// Three-dot menu icon
function DotsIcon() {
  return (
    <svg className="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
      <path d="M10 6a2 2 0 110-4 2 2 0 010 4zM10 12a2 2 0 110-4 2 2 0 010 4zM10 18a2 2 0 110-4 2 2 0 010 4z" />
    </svg>
  );
}

// Settings category icons
function FolderIcon() {
  return (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={1.5}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
    </svg>
  );
}

function AppsIcon() {
  return (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={1.5}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M6.429 9.75L2.25 12l4.179 2.25m0-4.5l5.571 3 5.571-3m-11.142 0L2.25 7.5 12 2.25l9.75 5.25-4.179 2.25m0 0L21.75 12l-4.179 2.25m0 0l4.179 2.25L12 21.75 2.25 16.5l4.179-2.25m11.142 0l-5.571 3-5.571-3" />
    </svg>
  );
}

type SettingsCategory = 'folders' | 'apps';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  folders: MonitoredFolder[];
  onRefreshFolders: () => Promise<void>;
  onSettingsChange: () => void;
}

// Settings Modal Component with dual-panel layout
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
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
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
  const [showHeaderMenu, setShowHeaderMenu] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [headerMenuPosition, setHeaderMenuPosition] = useState<{ top: number; left: number } | null>(null);
  const [defaultTerminal, setDefaultTerminal] = useState<TerminalApp | null>(null);
  const headerMenuRef = useRef<HTMLDivElement>(null);
  const headerMenuButtonRef = useRef<HTMLButtonElement>(null);

  const loadFolders = useCallback(async () => {
    try {
      const loaded = await api.getMonitoredFolders();
      setFolders(loaded || []);
    } catch (err) {
      console.error('Failed to load folders:', err);
    }
  }, []);

  const loadTerminalSettings = useCallback(async () => {
    try {
      const [settings, terminals] = await Promise.all([
        api.getAppSettings(),
        api.getAvailableTerminals(),
      ]);
      if (settings.defaultTerminal) {
        const terminal = terminals.find(t => t.id === settings.defaultTerminal);
        setDefaultTerminal(terminal ?? null);
      } else {
        setDefaultTerminal(null);
      }
    } catch (err) {
      console.error('Failed to load terminal settings:', err);
    }
  }, []);

  useEffect(() => {
    loadFolders();
    loadTerminalSettings();
  }, [loadFolders, loadTerminalSettings]);

  // Close header menu when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (headerMenuRef.current && !headerMenuRef.current.contains(event.target as Node)) {
        setShowHeaderMenu(false);
      }
    }
    if (showHeaderMenu) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [showHeaderMenu]);

  const openHeaderMenu = () => {
    if (headerMenuButtonRef.current) {
      const rect = headerMenuButtonRef.current.getBoundingClientRect();
      setHeaderMenuPosition({
        top: rect.bottom + 4,
        left: rect.right - 120,
      });
    }
    setShowHeaderMenu(true);
  };

  return (
    <div className="h-screen flex flex-col bg-dark-bg text-text-primary overflow-hidden">

      {/* Custom Title Bar */}
      <header
        data-tauri-drag-region
        className="flex-shrink-0 h-11 flex items-center justify-between px-3 bg-dark-surface/50 border-b border-dark-border"
        style={{ paddingLeft: '80px' }} // Space for macOS traffic lights
      >
        <span
          data-tauri-drag-region
          className="text-xs font-medium text-text-secondary select-none pointer-events-none"
        >
          Git Projects Manager
        </span>
        <button
          ref={headerMenuButtonRef}
          onClick={() => {
            if (showHeaderMenu) {
              setShowHeaderMenu(false);
            } else {
              openHeaderMenu();
            }
          }}
          className={`p-1.5 rounded hover:bg-dark-border transition-colors text-text-secondary hover:text-text-primary ${showHeaderMenu ? 'bg-dark-border' : ''}`}
        >
          <DotsIcon />
        </button>
      </header>

      {/* Header Menu Dropdown */}
      {showHeaderMenu && headerMenuPosition && (
        <div
          ref={headerMenuRef}
          className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-1 min-w-[120px]"
          style={{ top: headerMenuPosition.top, left: headerMenuPosition.left }}
        >
          <button
            onClick={() => {
              setShowHeaderMenu(false);
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
        />
      </main>

      {/* Settings Modal */}
      <SettingsModal
        isOpen={showSettings}
        onClose={() => setShowSettings(false)}
        folders={folders}
        onRefreshFolders={loadFolders}
        onSettingsChange={loadTerminalSettings}
      />
    </div>
  );
}

export default App;
