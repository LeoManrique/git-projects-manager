import React, { useState } from 'react';
import { MonitoredFolder, ScanResult, RepoStatus } from '../types';

interface ScanResultsProps {
  folders: MonitoredFolder[];
  scanResults: Map<string, ScanResult>;
  onScan: (path: string) => Promise<void>;
  onScanAll: () => Promise<void>;
}

export const ScanResults: React.FC<ScanResultsProps> = ({
  folders,
  scanResults,
  onScan,
  onScanAll,
}) => {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [scanning, setScanning] = useState<Set<string>>(new Set());
  const [scanningAll, setScanningAll] = useState(false);

  const toggleFolder = (folderId: string) => {
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(folderId)) {
      newExpanded.delete(folderId);
    } else {
      newExpanded.add(folderId);
    }
    setExpandedFolders(newExpanded);
  };

  const handleScan = async (folder: MonitoredFolder) => {
    setScanning(prev => new Set(prev).add(folder.id));
    try {
      await onScan(folder.path);
    } finally {
      setScanning(prev => {
        const newSet = new Set(prev);
        newSet.delete(folder.id);
        return newSet;
      });
    }
  };

  const handleScanAll = async () => {
    setScanningAll(true);
    try {
      await onScanAll();
    } finally {
      setScanningAll(false);
    }
  };

  const getStatusIcon = (status: 'clean' | 'changes' | 'unpushed' | 'error') => {
    switch (status) {
      case 'clean':
        return '✓';
      case 'changes':
        return '●';
      case 'unpushed':
        return '↑';
      case 'error':
        return '✗';
    }
  };

  const getStatusColor = (status: 'clean' | 'changes' | 'unpushed' | 'error') => {
    switch (status) {
      case 'clean':
        return 'status-clean';
      case 'changes':
        return 'status-changes';
      case 'unpushed':
        return 'status-unpushed';
      case 'error':
        return 'status-error';
    }
  };

  const renderRepoStatus = (repo: RepoStatus, status: 'clean' | 'changes' | 'unpushed' | 'error') => {
    const relativePath = repo.path.split(/[/\\]/).pop() || repo.path;

    return (
      <div key={repo.path} className={`repo-item ${getStatusColor(status)}`}>
        <span className="repo-status-icon">{getStatusIcon(status)}</span>
        <span className="repo-path" title={repo.path}>{relativePath}</span>
        {repo.branch && (
          <span className="repo-branch">{repo.branch}</span>
        )}
        {repo.errorMessage && (
          <span className="repo-error" title={repo.errorMessage}>
            {repo.errorMessage}
          </span>
        )}
      </div>
    );
  };

  const renderScanResult = (folder: MonitoredFolder, result: ScanResult | undefined) => {
    const isExpanded = expandedFolders.has(folder.id);
    const isScanning = scanning.has(folder.id);

    if (isScanning) {
      return (
        <div className="scan-result-empty">
          <p>Scanning in progress...</p>
        </div>
      );
    }

    if (!result) {
      return (
        <div className="scan-result-empty">
          <p>No scan results yet</p>
        </div>
      );
    }

    const totalClean = result.clean.length;
    const totalChanges = result.withChanges.length;
    const totalUnpushed = result.withUnpushed.length;
    const totalErrors = result.errors.length;

    return (
      <div className="scan-result">
        <div className="scan-summary">
          <span className="summary-item">
            Total: <strong>{result.totalRepositories}</strong>
          </span>
          {totalClean > 0 && (
            <span className={`summary-item ${getStatusColor('clean')}`}>
              Clean: <strong>{totalClean}</strong>
            </span>
          )}
          {totalChanges > 0 && (
            <span className={`summary-item ${getStatusColor('changes')}`}>
              Changes: <strong>{totalChanges}</strong>
            </span>
          )}
          {totalUnpushed > 0 && (
            <span className={`summary-item ${getStatusColor('unpushed')}`}>
              Unpushed: <strong>{totalUnpushed}</strong>
            </span>
          )}
          {totalErrors > 0 && (
            <span className={`summary-item ${getStatusColor('error')}`}>
              Errors: <strong>{totalErrors}</strong>
            </span>
          )}
          <span className="summary-item execution-time">
            Time: <strong>{result.executionTime.toFixed(2)}s</strong>
          </span>
        </div>

        {isExpanded && (
          <div className="repo-list">
            {result.withChanges.length > 0 && (
              <div className="repo-section">
                <h4 className={getStatusColor('changes')}>
                  {getStatusIcon('changes')} Uncommitted Changes ({result.withChanges.length})
                </h4>
                <div className="repo-items">
                  {result.withChanges.map(repo => renderRepoStatus(repo, 'changes'))}
                </div>
              </div>
            )}

            {result.withUnpushed.length > 0 && (
              <div className="repo-section">
                <h4 className={getStatusColor('unpushed')}>
                  {getStatusIcon('unpushed')} Unpushed Commits ({result.withUnpushed.length})
                </h4>
                <div className="repo-items">
                  {result.withUnpushed.map(repo => renderRepoStatus(repo, 'unpushed'))}
                </div>
              </div>
            )}

            {result.errors.length > 0 && (
              <div className="repo-section">
                <h4 className={getStatusColor('error')}>
                  {getStatusIcon('error')} Errors ({result.errors.length})
                </h4>
                <div className="repo-items">
                  {result.errors.map(repo => renderRepoStatus(repo, 'error'))}
                </div>
              </div>
            )}

            {result.clean.length > 0 && (
              <div className="repo-section">
                <h4 className={getStatusColor('clean')}>
                  {getStatusIcon('clean')} Clean ({result.clean.length})
                </h4>
                <div className="repo-items">
                  {result.clean.map(repo => renderRepoStatus(repo, 'clean'))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="scan-results">
      <div className="header">
        <h2>Repository Scan Results</h2>
        <button
          onClick={handleScanAll}
          disabled={scanningAll || folders.length === 0}
          className="btn btn-primary"
        >
          {scanningAll ? 'Scanning...' : 'Scan All'}
        </button>
      </div>

      {folders.length === 0 ? (
        <div className="empty-state">
          <p>No folders to scan.</p>
          <p>Go to "Manage Folders" to add folders to monitor.</p>
        </div>
      ) : (
        <div className="folder-scan-list">
          {folders.map(folder => {
            const result = scanResults.get(folder.path);
            const isExpanded = expandedFolders.has(folder.id);
            const isScanning = scanning.has(folder.id);

            return (
              <div key={folder.id} className="folder-scan-item">
                <div className="folder-scan-header">
                  <button
                    className="folder-toggle"
                    onClick={() => toggleFolder(folder.id)}
                    disabled={!result}
                  >
                    <span className="toggle-icon">
                      {!result ? '○' : isExpanded ? '▼' : '▶'}
                    </span>
                    <span className="folder-name">{folder.name}</span>
                  </button>

                  <div className="folder-scan-actions">
                    <span className="folder-path">{folder.path}</span>
                    <button
                      onClick={() => handleScan(folder)}
                      disabled={isScanning || scanningAll}
                      className="btn btn-secondary"
                    >
                      {isScanning ? 'Scanning...' : 'Scan'}
                    </button>
                  </div>
                </div>

                {renderScanResult(folder, result)}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};