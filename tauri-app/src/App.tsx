import { useState, useEffect } from 'react';
import { MonitoredFolder, ScanResult } from './types';
import { api } from './lib/api';
import './App.css';

function App() {
  const [folders, setFolders] = useState<MonitoredFolder[]>([]);
  const [scanResults, setScanResults] = useState<Map<string, ScanResult>>(new Map());
  const [isScanning, setIsScanning] = useState(false);

  useEffect(() => {
    loadFolders();
  }, []);

  const loadFolders = async () => {
    try {
      const loadedFolders = await api.getMonitoredFolders();
      setFolders(loadedFolders);
    } catch (error) {
      console.error('Failed to load folders:', error);
    }
  };

  const handleAddFolder = async () => {
    try {
      const path = await api.browseFolder();
      if (path) {
        const folderName = path.split(/[\\/]/).pop() || path;
        await api.addMonitoredFolder(path, folderName);
        await loadFolders();
      }
    } catch (error) {
      console.error('Failed to add folder:', error);
    }
  };

  const handleDeleteFolder = async (id: string) => {
    try {
      await api.deleteMonitoredFolder(id);
      await loadFolders();
    } catch (error) {
      console.error('Failed to delete folder:', error);
    }
  };

  const handleScan = async (path: string) => {
    try {
      setIsScanning(true);
      const result = await api.scanFolder(path);
      setScanResults(prev => new Map(prev).set(path, result));
    } catch (error) {
      console.error('Scan failed:', error);
    } finally {
      setIsScanning(false);
    }
  };

  const handleScanAll = async () => {
    try {
      setIsScanning(true);
      await api.cancelScan();

      for (const folder of folders) {
        await handleScan(folder.path);
      }
    } catch (error) {
      console.error('Scan all failed:', error);
    } finally {
      setIsScanning(false);
    }
  };

  return (
    <div className="app">
      <h1>Git Projects Manager</h1>

      <div className="controls">
        <button onClick={handleAddFolder}>Add Folder</button>
        <button onClick={handleScanAll} disabled={isScanning || folders.length === 0}>
          {isScanning ? 'Scanning...' : 'Scan All'}
        </button>
      </div>

      <div className="folders-section">
        <h2>Monitored Folders</h2>
        {folders.length === 0 ? (
          <p className="empty-state">No folders monitored. Add a folder to get started.</p>
        ) : (
          <div className="folders-list">
            {folders.map((folder) => (
              <div key={folder.id} className="folder-item">
                <div className="folder-info">
                  <strong>{folder.name}</strong>
                  <span className="folder-path">{folder.path}</span>
                </div>
                <div className="folder-actions">
                  <button onClick={() => handleScan(folder.path)} disabled={isScanning}>
                    Scan
                  </button>
                  <button onClick={() => handleDeleteFolder(folder.id)}>
                    Delete
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="results-section">
        <h2>Scan Results</h2>
        {scanResults.size === 0 ? (
          <p className="empty-state">No scan results yet. Click "Scan All" or scan individual folders.</p>
        ) : (
          Array.from(scanResults.entries()).map(([path, result]) => (
            <div key={path} className="scan-result">
              <h3>{path}</h3>
              <div className="scan-stats">
                <span>Total: {result.totalRepositories}</span>
                <span>Changes: {result.withChanges.length}</span>
                <span>Unpushed: {result.withUnpushed.length}</span>
                <span>Clean: {result.clean.length}</span>
                <span>Errors: {result.errors.length}</span>
                <span>Time: {result.executionTime.toFixed(2)}s</span>
              </div>

              {result.withChanges.length > 0 && (
                <div className="repo-list">
                  <h4>Repositories with Changes</h4>
                  {result.withChanges.map((repo, idx) => (
                    <div key={idx} className="repo-item changes">
                      <span>{repo.path}</span>
                      {repo.branch && <span className="branch">{repo.branch}</span>}
                    </div>
                  ))}
                </div>
              )}

              {result.withUnpushed.length > 0 && (
                <div className="repo-list">
                  <h4>Repositories with Unpushed Commits</h4>
                  {result.withUnpushed.map((repo, idx) => (
                    <div key={idx} className="repo-item unpushed">
                      <span>{repo.path}</span>
                      {repo.branch && <span className="branch">{repo.branch}</span>}
                    </div>
                  ))}
                </div>
              )}

              {result.errors.length > 0 && (
                <div className="repo-list">
                  <h4>Errors</h4>
                  {result.errors.map((repo, idx) => (
                    <div key={idx} className="repo-item error">
                      <span>{repo.path}</span>
                      {repo.errorMessage && <span className="error-msg">{repo.errorMessage}</span>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

export default App;
