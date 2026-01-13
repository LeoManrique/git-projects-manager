export interface TerminalApp {
  id: string;
  name: string;
  displayName: string;
  path: string;
}

export interface EditorApp {
  id: string;
  name: string;
  displayName: string;
  path: string;
}

export interface AppSettings {
  defaultTerminal?: string;
  defaultEditor?: string;
}
