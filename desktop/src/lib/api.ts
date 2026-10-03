import { Channel, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
  MonitoredFolder,
  FolderFormValues,
  ScanResult,
  AppSettings,
  TerminalApp,
  EditorApp,
  KanbanState,
  KanbanRefresh,
  GhAuthStatus,
  GitCleanSettings,
  GitCleanResult,
  SyncUser,
  LogLevel,
} from '../types';

/**
 * A folder's settings as command arguments, listed by name so that an extra
 * property on the caller's object (an `id`, when the edit form was seeded from
 * an existing folder) never reaches the payload.
 */
function folderArgs({ path, name, onlyLocalChecks, detectUninitialized }: FolderFormValues) {
  return { path, name, onlyLocalChecks, detectUninitialized };
}

export const api = {
  // Folder management
  async getMonitoredFolders(): Promise<MonitoredFolder[]> {
    return await invoke('get_monitored_folders');
  },

  async addMonitoredFolder(values: FolderFormValues): Promise<MonitoredFolder> {
    return await invoke('add_monitored_folder', folderArgs(values));
  },

  async updateMonitoredFolder(id: string, values: FolderFormValues): Promise<void> {
    await invoke('update_monitored_folder', { id, ...folderArgs(values) });
  },

  async deleteMonitoredFolder(id: string): Promise<void> {
    await invoke('delete_monitored_folder', { id });
  },

  // Scanning
  /** Resolves with the final snapshot; `onSnapshot` gets the ones before it. */
  async scanFolder(
    folder: MonitoredFolder,
    onSnapshot: (snapshot: ScanResult) => void,
  ): Promise<ScanResult> {
    const channel = new Channel<ScanResult>();
    channel.onmessage = onSnapshot;
    return await invoke('scan_folder', {
      path: folder.path,
      onlyLocalChecks: folder.onlyLocalChecks,
      detectUninitialized: folder.detectUninitialized,
      onSnapshot: channel,
    });
  },

  /** Null when the folder was never scanned. */
  async recheckRepos(folder: MonitoredFolder, repos: string[]): Promise<ScanResult | null> {
    return await invoke('recheck_repos', {
      folder: folder.path,
      repos,
      onlyLocalChecks: folder.onlyLocalChecks,
    });
  },

  // Git operations
  async pullRepo(path: string): Promise<string> {
    return await invoke('pull_repo', { path });
  },

  async cleanRepo(path: string): Promise<GitCleanResult> {
    return await invoke('clean_repo', { path });
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
  },

  async setDefaultEditor(editorId: string | null): Promise<void> {
    await invoke('set_default_editor', { editorId });
  },

  async getAvailableEditors(): Promise<EditorApp[]> {
    return await invoke('get_available_editors');
  },

  async openInEditor(path: string, editorId: string): Promise<void> {
    await invoke('open_in_editor', { path, editorId });
  },

  async openInLmsGithub(path: string): Promise<void> {
    await invoke('open_in_lms_github', { path });
  },

  // Git Clean Settings
  async getGitCleanSettings(): Promise<GitCleanSettings> {
    return await invoke('get_git_clean_settings');
  },

  async setGitCleanSettings(settings: GitCleanSettings): Promise<void> {
    await invoke('set_git_clean_settings', { settings });
  },

  // Kanban
  async checkGhAuth(): Promise<GhAuthStatus> {
    return await invoke('check_gh_auth');
  },

  async refreshKanban(): Promise<KanbanRefresh> {
    return await invoke('refresh_kanban');
  },

  async loadKanbanLocal(): Promise<KanbanRefresh | null> {
    return await invoke('load_kanban_local');
  },

  async moveKanbanCard(nameWithOwner: string, toColumn: string): Promise<KanbanState> {
    return await invoke('move_kanban_card', { nameWithOwner, toColumn });
  },

  async updateKanbanNotes(nameWithOwner: string, notes: string | null): Promise<KanbanState> {
    return await invoke('update_kanban_notes', { nameWithOwner, notes });
  },

  async deleteGithubRepo(nameWithOwner: string): Promise<KanbanRefresh> {
    return await invoke('delete_github_repo', { nameWithOwner });
  },

  async openUrl(url: string): Promise<void> {
    await invoke('open_url', { url });
  },

  // Sync auth
  async signInWithGoogle(): Promise<SyncUser> {
    return await invoke('sign_in_with_google');
  },

  async signOut(): Promise<void> {
    await invoke('sign_out');
  },

  async getSyncUser(): Promise<SyncUser | null> {
    return await invoke('get_sync_user');
  },

  // Diagnostics log
  async logMessage(level: LogLevel, message: string): Promise<void> {
    await invoke('log_message', { level, message });
  },

  async getLogsFolder(): Promise<string> {
    return await invoke('get_logs_folder');
  },

  async openLogsFolder(): Promise<void> {
    await invoke('open_logs_folder');
  },
};
