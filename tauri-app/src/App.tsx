import { useState, useEffect, useCallback } from 'react';
import FolderManager from './components/FolderManager';
import ScanResults, { ScanResultsState } from './components/ScanResults';
import { api } from './lib/api';
import { MonitoredFolder } from './types';
import './App.css';

function App() {
  const [folders, setFolders] = useState<MonitoredFolder[]>([]);
  const [activeTab, setActiveTab] = useState<'manage' | 'scan'>('scan');
  const [scanState, setScanState] = useState<ScanResultsState>({
    results: {},
    expandedFolders: new Set(),
  });

  const loadFolders = useCallback(async () => {
    try {
      const loaded = await api.getMonitoredFolders();
      setFolders(loaded || []);
    } catch (err) {
      console.error('Failed to load folders:', err);
    }
  }, []);

  useEffect(() => {
    loadFolders();
  }, [loadFolders]);

  return (
    <div className="h-screen flex flex-col bg-dark-bg text-text-primary overflow-hidden">
      {/* Tab Navigation */}
      <nav className="flex-shrink-0 bg-dark-surface border-b border-dark-border px-3">
        <div className="flex">
          <button
            onClick={() => setActiveTab('scan')}
            className={`px-3 py-2 text-sm font-medium transition-colors relative ${
              activeTab === 'scan'
                ? 'text-text-primary'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            Scan Results
            {activeTab === 'scan' && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-blue" />
            )}
          </button>
          <button
            onClick={() => setActiveTab('manage')}
            className={`px-3 py-2 text-sm font-medium transition-colors relative ${
              activeTab === 'manage'
                ? 'text-text-primary'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            Manage Folders
            {activeTab === 'manage' && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-blue" />
            )}
          </button>
        </div>
      </nav>

      {/* Main Content */}
      <main className="flex-1 overflow-auto p-4">
        {activeTab === 'manage' && (
          <FolderManager folders={folders} onRefresh={loadFolders} />
        )}
        {activeTab === 'scan' && (
          <ScanResults
            folders={folders}
            scanState={scanState}
            onScanStateChange={setScanState}
          />
        )}
      </main>
    </div>
  );
}

export default App;
