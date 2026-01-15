import { useState, useEffect } from 'react';
import { api } from '../../lib/api';
import { TrashIcon } from '../icons';

interface GitCleanSettingsProps {
  onSettingsChange: () => void;
}

export default function GitCleanSettings({ onSettingsChange }: GitCleanSettingsProps) {
  const [excludePatterns, setExcludePatterns] = useState<string[]>([]);
  const [newPattern, setNewPattern] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      setIsLoading(true);
      setError('');
      const settings = await api.getGitCleanSettings();
      setExcludePatterns(settings.excludePatterns);
    } catch (err) {
      setError('Failed to load settings');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  const saveSettings = async (patterns: string[]) => {
    try {
      setIsSaving(true);
      setError('');
      await api.setGitCleanSettings({ excludePatterns: patterns });
      setExcludePatterns(patterns);
      onSettingsChange();
    } catch (err) {
      setError('Failed to save settings');
      console.error(err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleAddPattern = async () => {
    const trimmed = newPattern.trim();
    if (!trimmed) return;
    if (excludePatterns.includes(trimmed)) {
      setError('Pattern already exists');
      return;
    }
    const newPatterns = [...excludePatterns, trimmed];
    await saveSettings(newPatterns);
    setNewPattern('');
  };

  const handleRemovePattern = async (pattern: string) => {
    const newPatterns = excludePatterns.filter(p => p !== pattern);
    await saveSettings(newPatterns);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddPattern();
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <span className="w-4 h-4 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {error && (
        <div className="bg-accent-red/10 border border-accent-red/20 text-accent-red px-2.5 py-1.5 rounded text-xs">
          {error}
        </div>
      )}

      <section>
        <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2">
          About Git Clean
        </h3>
        <p className="text-text-muted text-xs mb-3">
          The "Clean" action removes ignored files from repositories (files matching patterns in .gitignore).
          This is useful for removing build artifacts, caches, and other generated files.
        </p>
        <p className="text-text-muted text-xs mb-3">
          Command: <code className="bg-dark-border px-1.5 py-0.5 rounded font-mono">git clean -fdX</code>
        </p>
      </section>

      <section>
        <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2">
          Exclude Patterns
        </h3>
        <p className="text-text-muted text-xs mb-3">
          Files matching these patterns will be preserved during cleaning. Use glob patterns like <code className="bg-dark-border px-1 py-0.5 rounded font-mono text-[10px]">.env*</code> or <code className="bg-dark-border px-1 py-0.5 rounded font-mono text-[10px]">*.key</code>.
        </p>

        {/* Pattern list */}
        <div className="space-y-1.5 mb-3">
          {excludePatterns.length === 0 ? (
            <p className="text-text-muted text-xs text-center py-4 border border-dashed border-dark-border rounded">
              No exclude patterns configured. All ignored files will be removed.
            </p>
          ) : (
            excludePatterns.map((pattern) => (
              <div
                key={pattern}
                className="flex items-center justify-between px-2.5 py-1.5 bg-dark-surface border border-dark-border rounded group"
              >
                <code className="text-xs font-mono text-text-primary">{pattern}</code>
                <button
                  onClick={() => handleRemovePattern(pattern)}
                  disabled={isSaving}
                  className="p-1 rounded hover:bg-dark-border text-text-muted hover:text-accent-red transition-colors opacity-0 group-hover:opacity-100 disabled:opacity-40"
                  title="Remove pattern"
                >
                  <TrashIcon />
                </button>
              </div>
            ))
          )}
        </div>

        {/* Add new pattern */}
        <div className="flex gap-1.5">
          <input
            type="text"
            value={newPattern}
            onChange={(e) => setNewPattern(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="e.g., .env*, *.key, .vscode/"
            disabled={isSaving}
            className="flex-1 px-2.5 py-1.5 bg-dark-bg border border-dark-border rounded text-text-primary text-xs font-mono placeholder-text-muted focus:outline-none focus:border-accent-blue/50 disabled:opacity-40"
          />
          <button
            onClick={handleAddPattern}
            disabled={isSaving || !newPattern.trim()}
            className="px-3 py-1.5 rounded bg-accent-blue/90 hover:bg-accent-blue text-white text-xs font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          >
            Add
          </button>
        </div>
      </section>
    </div>
  );
}
