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
      <nav className="flex-shrink-0 bg-dark-surface/50 border-b border-dark-border">
        <div className="flex gap-1 px-2 py-1">
          <button
            onClick={() => setActiveTab('scan')}
            className={`px-3 py-1 text-xs font-medium rounded transition-colors ${
              activeTab === 'scan'
                ? 'bg-dark-elevated text-text-primary'
                : 'text-text-secondary hover:text-text-primary hover:bg-dark-borderSubtle'
            }`}
          >
            Scan Results
          </button>
          <button
            onClick={() => setActiveTab('manage')}
            className={`px-3 py-1 text-xs font-medium rounded transition-colors ${
              activeTab === 'manage'
                ? 'bg-dark-elevated text-text-primary'
                : 'text-text-secondary hover:text-text-primary hover:bg-dark-borderSubtle'
            }`}
          >
            Manage Folders
          </button>
        </div>
      </nav>

      {/* Main Content */}
      <main className="flex-1 overflow-auto p-3">
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
