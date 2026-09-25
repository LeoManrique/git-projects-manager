import { useState, useEffect } from 'react';
import { api } from '../../lib/api';
import { logError } from '../../lib/log';

/** Where the diagnostics log lives, and a way to get to it (FRONTEND.md §6.4). */
export default function LogsSettings() {
  const [folder, setFolder] = useState('');
  const [error, setError] = useState('');

  useEffect(() => {
    api.getLogsFolder().then(setFolder, (err) => logError('Failed to find the logs folder', err));
  }, []);

  const handleOpen = async () => {
    setError('');
    try {
      await api.openLogsFolder();
    } catch (err) {
      setError('Failed to open the logs folder');
      logError('Failed to open the logs folder', err);
    }
  };

  return (
    <div className="space-y-6">
      {error && (
        <div className="bg-accent-red/10 border border-accent-red/20 text-accent-red px-2.5 py-1.5 rounded text-xs">
          {error}
        </div>
      )}

      <section>
        <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2">
          Diagnostics Log
        </h3>
        <p className="text-text-muted text-xs mb-3 leading-relaxed">
          Every git command that fails or runs slowly, every error shown in the app, and every
          scan's start and end are written here. One file per day, kept for two weeks.
        </p>
        {folder && (
          <p className="text-text-primary text-xs font-mono mb-3 break-all select-text">{folder}</p>
        )}
        <button
          onClick={handleOpen}
          className="px-3 py-1.5 text-xs rounded border border-dark-border bg-dark-surface hover:bg-dark-borderSubtle text-text-primary transition-colors"
        >
          Open Logs Folder
        </button>
      </section>
    </div>
  );
}
