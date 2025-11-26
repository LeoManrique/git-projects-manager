import { useState } from 'react';
import {
  AddMonitoredFolder,
  UpdateMonitoredFolder,
  DeleteMonitoredFolder,
} from '../../wailsjs/go/main/App';

export interface MonitoredFolder {
  id: string;
  path: string;
  name: string;
}

interface FolderManagerProps {
  folders: MonitoredFolder[];
  onRefresh: () => Promise<void>;
}

export default function FolderManager({ folders, onRefresh }: FolderManagerProps) {
  const [showAddForm, setShowAddForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formPath, setFormPath] = useState('');
  const [formName, setFormName] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  const resetForm = () => {
    setFormPath('');
    setFormName('');
    setEditingId(null);
    setShowAddForm(false);
    setError('');
  };

  const executeOperation = async (operation: () => Promise<any>, errorMsg: string) => {
    try {
      setIsLoading(true);
      await operation();
      await onRefresh();
      resetForm();
    } catch (err) {
      setError(errorMsg);
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleAdd = () => {
    if (!formPath.trim() || !formName.trim()) {
      setError('Path and name are required');
      return;
    }
    executeOperation(() => AddMonitoredFolder(formPath, formName), 'Failed to add folder');
  };

  const handleUpdate = () => {
    if (!formPath.trim() || !formName.trim() || !editingId) {
      setError('Path and name are required');
      return;
    }
    executeOperation(() => UpdateMonitoredFolder(editingId, formPath, formName), 'Failed to update folder');
  };

  const handleDelete = (id: string) => {
    if (!window.confirm('Are you sure you want to delete this folder?')) return;
    executeOperation(() => DeleteMonitoredFolder(id), 'Failed to delete folder');
  };

  const toggleForm = () => {
    if (showAddForm && !editingId) {
      setShowAddForm(false);
    } else {
      resetForm();
      setShowAddForm(true);
    }
  };

  const handleEdit = (folder: MonitoredFolder) => {
    setEditingId(folder.id);
    setFormPath(folder.path);
    setFormName(folder.name);
    setShowAddForm(true);
    setError('');
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex-shrink-0 flex justify-between items-center mb-3">
        <h2 className="text-base font-semibold text-text-primary">Monitored Folders</h2>
        <button
          onClick={toggleForm}
          className="bg-accent-blue hover:brightness-110 text-white text-xs font-medium py-1.5 px-3 rounded transition-all"
        >
          {showAddForm && !editingId ? 'Cancel' : 'Add Folder'}
        </button>
      </div>

      {error && (
        <div className="flex-shrink-0 bg-accent-red/10 border border-accent-red/30 text-accent-red px-3 py-2 rounded text-xs mb-3">
          {error}
        </div>
      )}

      {showAddForm && (
        <div className="flex-shrink-0 bg-dark-surface rounded border border-dark-border p-3 mb-3">
          <div className="space-y-3">
            <div>
              <label className="block text-text-secondary text-xs font-medium mb-1.5">
                Folder Path
              </label>
              <input
                type="text"
                value={formPath}
                onChange={(e) => setFormPath(e.target.value)}
                placeholder="e.g., C:\Users\YourName\GitProjects"
                className="w-full px-2.5 py-1.5 bg-dark-bg border border-dark-border rounded text-text-primary text-sm placeholder-text-muted focus:outline-none focus:border-accent-blue"
              />
            </div>

            <div>
              <label className="block text-text-secondary text-xs font-medium mb-1.5">
                Display Name
              </label>
              <input
                type="text"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder="e.g., My Projects"
                className="w-full px-2.5 py-1.5 bg-dark-bg border border-dark-border rounded text-text-primary text-sm placeholder-text-muted focus:outline-none focus:border-accent-blue"
              />
            </div>

            <div className="flex gap-2 justify-end">
              <button
                onClick={resetForm}
                disabled={isLoading}
                className="px-3 py-1.5 rounded bg-dark-elevated hover:bg-dark-border text-text-secondary text-xs font-medium transition-colors disabled:opacity-50"
              >
                Cancel
              </button>
              <button
                onClick={editingId ? handleUpdate : handleAdd}
                disabled={isLoading}
                className="px-3 py-1.5 rounded bg-accent-blue hover:brightness-110 text-white text-xs font-medium transition-all disabled:opacity-50"
              >
                {isLoading ? 'Saving...' : editingId ? 'Update' : 'Add'}
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="flex-1 overflow-auto space-y-2">
        {folders.length === 0 ? (
          <p className="text-text-muted text-sm text-center py-8">No folders configured yet.</p>
        ) : (
          folders.map((folder) => (
            <div
              key={folder.id}
              className="flex justify-between items-center bg-dark-surface rounded border border-dark-border px-3 py-2.5 hover:bg-dark-elevated transition-colors"
            >
              <div className="flex-1 min-w-0">
                <h3 className="text-text-primary text-sm font-medium truncate">{folder.name}</h3>
                <p className="text-text-muted text-xs truncate">{folder.path}</p>
              </div>
              <div className="flex gap-2 ml-3 flex-shrink-0">
                <button
                  onClick={() => handleEdit(folder)}
                  disabled={isLoading}
                  className="px-2 py-1 rounded bg-accent-blue/20 hover:bg-accent-blue/30 text-accent-blue text-xs font-medium transition-colors disabled:opacity-50"
                >
                  Edit
                </button>
                <button
                  onClick={() => handleDelete(folder.id)}
                  disabled={isLoading}
                  className="px-2 py-1 rounded bg-accent-red/20 hover:bg-accent-red/30 text-accent-red text-xs font-medium transition-colors disabled:opacity-50"
                >
                  Delete
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
