import { useState, useEffect } from 'react';
import { api } from '../../lib/api';
import { TerminalApp, EditorApp } from '../../types';
import { Select, SelectOption } from '../ui/Select';

interface DefaultAppsSettingsProps {
  onSettingsChange: () => void;
}

export default function DefaultAppsSettings({ onSettingsChange }: DefaultAppsSettingsProps) {
  const [terminals, setTerminals] = useState<TerminalApp[]>([]);
  const [editors, setEditors] = useState<EditorApp[]>([]);
  const [selectedTerminal, setSelectedTerminal] = useState<string | null>(null);
  const [selectedEditor, setSelectedEditor] = useState<string | null>(null);
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

      const [availableTerminals, availableEditors, settings] = await Promise.all([
        api.getAvailableTerminals(),
        api.getAvailableEditors(),
        api.getAppSettings(),
      ]);

      setTerminals(availableTerminals);
      setEditors(availableEditors);
      setSelectedTerminal(settings.defaultTerminal ?? null);
      setSelectedEditor(settings.defaultEditor ?? null);
    } catch (err) {
      setError('Failed to load settings');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleTerminalChange = async (terminalId: string) => {
    try {
      setIsSaving(true);
      setError('');
      await api.setDefaultTerminal(terminalId);
      setSelectedTerminal(terminalId);
      onSettingsChange();
    } catch (err) {
      setError('Failed to save setting');
      console.error(err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleEditorChange = async (editorId: string) => {
    try {
      setIsSaving(true);
      setError('');
      await api.setDefaultEditor(editorId);
      setSelectedEditor(editorId);
      onSettingsChange();
    } catch (err) {
      setError('Failed to save setting');
      console.error(err);
    } finally {
      setIsSaving(false);
    }
  };

  const terminalOptions: SelectOption<string>[] = terminals.map((terminal) => ({
    value: terminal.id,
    label: terminal.displayName,
  }));

  const editorOptions: SelectOption<string>[] = editors.map((editor) => ({
    value: editor.id,
    label: editor.displayName,
  }));

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
          Default Terminal
        </h3>
        <p className="text-text-muted text-xs mb-3">
          Select which terminal application to use when opening repositories.
        </p>

        <Select
          options={terminalOptions}
          value={selectedTerminal}
          onChange={handleTerminalChange}
          placeholder="Choose a terminal..."
          disabled={isSaving}
        />

        {terminals.length === 0 && (
          <p className="text-text-muted text-xs text-center py-4">
            No terminal applications found on your system.
          </p>
        )}
      </section>

      <section>
        <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2">
          Default Editor
        </h3>
        <p className="text-text-muted text-xs mb-3">
          Select which code editor to use when opening repositories.
        </p>

        <Select
          options={editorOptions}
          value={selectedEditor}
          onChange={handleEditorChange}
          placeholder="Choose an editor..."
          disabled={isSaving}
        />

        {editors.length === 0 && (
          <p className="text-text-muted text-xs text-center py-4">
            No code editors found on your system.
          </p>
        )}
      </section>
    </div>
  );
}
