import { useState, useEffect } from 'react';
import { api } from '../../lib/api';
import { BrowseFolderIcon } from '../icons';

interface FolderFormProps {
  initialPath?: string;
  initialName?: string;
  onSubmit: (path: string, name: string) => Promise<void>;
  onCancel: () => void;
  submitLabel: string;
  isLoading: boolean;
  variant?: 'standalone' | 'inline';
}

export function FolderForm({
  initialPath = '',
  initialName = '',
  onSubmit,
  onCancel,
  submitLabel,
  isLoading,
  variant = 'standalone',
}: FolderFormProps) {
  const [formPath, setFormPath] = useState(initialPath);
  const [formName, setFormName] = useState(initialName);
  const [error, setError] = useState('');

  // Reset form when initial values change
  useEffect(() => {
    setFormPath(initialPath);
    setFormName(initialName);
  }, [initialPath, initialName]);

  const handleBrowse = async () => {
    try {
      const path = await api.browseFolder();
      if (path) {
        setFormPath(path);
      }
    } catch (err) {
      console.error('Failed to browse folder:', err);
    }
  };

  const handleSubmit = async () => {
    if (!formPath.trim() || !formName.trim()) {
      setError('Path and name are required');
      return;
    }
    setError('');
    await onSubmit(formPath, formName);
  };

  const containerClass = variant === 'standalone'
    ? 'bg-dark-surface rounded border border-dark-border p-2.5 mb-2'
    : 'bg-dark-bg/50 border-t border-dark-border px-2.5 py-2.5';

  const inputBgClass = variant === 'standalone' ? 'bg-dark-bg' : 'bg-dark-surface';

  return (
    <div className={containerClass}>
      {error && (
        <div className="bg-accent-red/10 border border-accent-red/20 text-accent-red px-2.5 py-1.5 rounded text-xs mb-2.5">
          {error}
        </div>
      )}
      <div className="space-y-2.5">
        <div>
          <label className="block text-text-secondary text-xs font-medium mb-1 uppercase tracking-wider">
            Folder Path
          </label>
          <div className="flex gap-1.5">
            <input
              type="text"
              value={formPath}
              onChange={(e) => setFormPath(e.target.value)}
              placeholder="e.g., /Users/YourName/Projects"
              className={`flex-1 px-2 py-1.5 ${inputBgClass} border border-dark-border rounded text-text-primary text-xs font-mono placeholder-text-muted focus:outline-none focus:border-accent-blue/50`}
            />
            <button
              type="button"
              onClick={handleBrowse}
              className="px-2 py-1 bg-dark-elevated hover:bg-dark-borderStrong border border-dark-border rounded text-text-secondary hover:text-text-primary transition-colors"
              title="Browse for folder"
            >
              <BrowseFolderIcon />
            </button>
          </div>
        </div>

        <div>
          <label className="block text-text-secondary text-xs font-medium mb-1 uppercase tracking-wider">
            Display Name
          </label>
          <input
            type="text"
            value={formName}
            onChange={(e) => setFormName(e.target.value)}
            placeholder="e.g., My Projects"
            className={`w-full px-2 py-1.5 ${inputBgClass} border border-dark-border rounded text-text-primary text-xs placeholder-text-muted focus:outline-none focus:border-accent-blue/50`}
          />
        </div>

        <div className="flex gap-1.5 justify-end pt-0.5">
          <button
            onClick={onCancel}
            disabled={isLoading}
            className="px-2.5 py-1 rounded bg-dark-borderStrong hover:bg-dark-elevated text-text-secondary text-xs font-medium transition-colors disabled:opacity-40"
          >
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            disabled={isLoading}
            className="px-2.5 py-1 rounded bg-accent-blue/90 hover:bg-accent-blue text-white text-xs font-medium transition-colors disabled:opacity-40"
          >
            {isLoading ? `${submitLabel}...` : submitLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
