import { useState, useEffect } from 'react';
import { api } from '../../lib/api';
import { TerminalApp } from '../../types';
import { Select, SelectOption } from '../ui/Select';

interface DefaultAppsSettingsProps {
  onSettingsChange: () => void;
}

export default function DefaultAppsSettings({ onSettingsChange }: DefaultAppsSettingsProps) {
  const [terminals, setTerminals] = useState<TerminalApp[]>([]);
  const [selectedTerminal, setSelectedTerminal] = useState<string | null>(null);
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

      const [availableTerminals, settings] = await Promise.all([
        api.getAvailableTerminals(),
        api.getAppSettings(),
      ]);

      setTerminals(availableTerminals);
      setSelectedTerminal(settings.defaultTerminal ?? null);
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

  const terminalOptions: SelectOption<string>[] = terminals.map((terminal) => ({
    value: terminal.id,
    label: terminal.name,
  }));

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <span className="w-4 h-4 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
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
    </div>
  );
}
