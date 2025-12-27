import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { MonitoredFolder, ScanResult, AppSettings, TerminalApp } from '../types';

export const api = {
  // Folder management
  async getMonitoredFolders(): Promise<MonitoredFolder[]> {
    return await invoke('get_monitored_folders');
  },

  async addMonitoredFolder(path: string, name: string): Promise<MonitoredFolder> {
    return await invoke('add_monitored_folder', { path, name });
  },

  async updateMonitoredFolder(id: string, path: string, name: string): Promise<void> {
    await invoke('update_monitored_folder', { id, path, name });
  },

  async deleteMonitoredFolder(id: string): Promise<void> {
    await invoke('delete_monitored_folder', { id });
  },

  // Scanning
  async scanFolder(path: string): Promise<ScanResult> {
    return await invoke('scan_folder', { path });
  },

  async cancelScan(): Promise<void> {
    await invoke('cancel_scan');
  },

  // Git operations
  async pullRepo(path: string): Promise<string> {
    return await invoke('pull_repo', { path });
  },

  // Dialog
  async browseFolder(): Promise<string | null> {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Folder to Monitor'
    });

    return selected as string | null;
  },

  // Settings
  async getAppSettings(): Promise<AppSettings> {
    return await invoke('get_app_settings');
  },

  async setDefaultTerminal(terminalId: string | null): Promise<void> {
    await invoke('set_default_terminal', { terminalId });
  },

  async getAvailableTerminals(): Promise<TerminalApp[]> {
    return await invoke('get_available_terminals');
  },

  async openInTerminal(path: string, terminalId: string): Promise<void> {
    await invoke('open_in_terminal', { path, terminalId });
  }
};
