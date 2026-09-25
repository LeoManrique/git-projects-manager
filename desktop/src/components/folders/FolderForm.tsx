import { useState } from 'react';
import { api } from '../../lib/api';
import { logError } from '../../lib/log';
import { FolderFormValues, NEW_FOLDER } from '../../types';
import { BrowseFolderIcon } from '../icons';

interface FolderFormProps {
  /** Existing values when editing; a new folder's defaults when adding. */
  initial?: FolderFormValues;
  onSubmit: (values: FolderFormValues) => Promise<void>;
  onCancel: () => void;
  submitLabel: string;
  isLoading: boolean;
  variant?: 'standalone' | 'inline';
}

export function FolderForm({
  initial = NEW_FOLDER,
  onSubmit,
  onCancel,
  submitLabel,
  isLoading,
  variant = 'standalone',
}: FolderFormProps) {
  // One state object rather than a field each: the form's shape is the folder's
  // shape, so adding a setting stays a single change here and in the caller.
  //
  // `initial` seeds the state once. Callers give the form a `key` tied to what
  // it edits, so pointing it at another folder remounts it with fresh state —
  // which is why there is no effect here syncing props into state.
  const [values, setValues] = useState<FolderFormValues>(initial);
  const [error, setError] = useState('');

  const set = <K extends keyof FolderFormValues>(key: K, value: FolderFormValues[K]) =>
    setValues((prev) => ({ ...prev, [key]: value }));

  const handleBrowse = async () => {
    try {
      const path = await api.browseFolder();
      if (path) set('path', path);
    } catch (err) {
      logError('Failed to browse folder', err);
    }
  };

  const handleSubmit = async () => {
    if (!values.path.trim() || !values.name.trim()) {
      setError('Path and name are required');
      return;
    }
    setError('');
    await onSubmit(values);
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
              value={values.path}
              onChange={(e) => set('path', e.target.value)}
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
            value={values.name}
            onChange={(e) => set('name', e.target.value)}
            placeholder="e.g., My Projects"
            className={`w-full px-2 py-1.5 ${inputBgClass} border border-dark-border rounded text-text-primary text-xs placeholder-text-muted focus:outline-none focus:border-accent-blue/50`}
          />
        </div>

        <div className="space-y-1.5">
          <Checkbox
            id="detectUninitialized"
            checked={values.detectUninitialized}
            onChange={(checked) => set('detectUninitialized', checked)}
            label="Only code projects"
          />
          <p className="text-text-muted text-[11px] leading-4 pl-5">
            Reports sub-folders without a git repository as Uninitialized.
          </p>

          <Checkbox
            id="onlyLocalChecks"
            checked={values.onlyLocalChecks}
            onChange={(checked) => set('onlyLocalChecks', checked)}
            label="Only local checks"
          />
          <p className="text-text-muted text-[11px] leading-4 pl-5">
            Skips remote fetch and push/pull checks for faster, offline-friendly
            scans.
          </p>
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

interface CheckboxProps {
  id: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
}

/** The form's per-setting switch, so both settings render identically. */
function Checkbox({ id, checked, onChange, label }: CheckboxProps) {
  return (
    <div className="flex items-center gap-2">
      <input
        type="checkbox"
        id={id}
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="w-3.5 h-3.5 bg-dark-bg border border-dark-border rounded accent-accent-blue focus:outline-none focus:ring-1 focus:ring-accent-blue/50"
      />
      <label htmlFor={id} className="text-text-primary text-xs cursor-pointer">
        {label}
      </label>
    </div>
  );
}
