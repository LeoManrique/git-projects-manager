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

  console.log('[FolderManager] Rendering with folders:', folders);

  const resetForm = () => {
    setFormPath('');
    setFormName('');
    setEditingId(null);
    setShowAddForm(false);
    setError('');
  };

  const handleAdd = async () => {
    if (!formPath.trim() || !formName.trim()) {
      setError('Path and name are required');
      return;
    }

    try {
      console.log('[FolderManager] Adding folder:', formPath, formName);
      setIsLoading(true);
      await AddMonitoredFolder(formPath, formName);
      console.log('[FolderManager] Folder added, refreshing...');
      await onRefresh();
      console.log('[FolderManager] Refresh complete');
      resetForm();
    } catch (err) {
      setError('Failed to add folder');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleUpdate = async () => {
    if (!formPath.trim() || !formName.trim() || !editingId) {
      setError('Path and name are required');
      return;
    }

    try {
      setIsLoading(true);
      await UpdateMonitoredFolder(editingId, formPath, formName);
      await onRefresh();
      resetForm();
    } catch (err) {
      setError('Failed to update folder');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!window.confirm('Are you sure you want to delete this folder?')) {
      return;
    }

    try {
      setIsLoading(true);
      await DeleteMonitoredFolder(id);
      await onRefresh();
    } catch (err) {
      setError('Failed to delete folder');
      console.error(err);
    } finally {
      setIsLoading(false);
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
    <div className="bg-dark-surface rounded-lg shadow-lg p-6 border border-dark-border">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold text-white">Monitored Folders</h2>
        <button
          onClick={() => {
            if (showAddForm && !editingId) {
              setShowAddForm(false);
            } else {
              resetForm();
              setShowAddForm(true);
            }
          }}
          className="bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2 px-4 rounded-lg transition-colors"
        >
          {showAddForm && !editingId ? 'Cancel' : '+ Add Folder'}
        </button>
      </div>

      {error && (
        <div className="bg-red-900 border border-red-700 text-red-100 px-4 py-3 rounded-lg mb-4">
          {error}
        </div>
      )}

      {showAddForm && (
        <div className="bg-dark-bg rounded-lg p-4 mb-6 border border-dark-border">
          <div className="space-y-4">
            <div>
              <label className="block text-gray-300 text-sm font-medium mb-2">
                Folder Path
              </label>
              <input
                type="text"
                value={formPath}
                onChange={(e) => setFormPath(e.target.value)}
                placeholder="e.g., C:\\Users\\YourName\\GitProjects"
                className="w-full px-4 py-2 bg-dark-bg border border-dark-border rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
              />
            </div>

            <div>
              <label className="block text-gray-300 text-sm font-medium mb-2">
                Display Name
              </label>
              <input
                type="text"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder="e.g., My Projects"
                className="w-full px-4 py-2 bg-dark-bg border border-dark-border rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
              />
            </div>

            <div className="flex gap-3 justify-end">
              <button
                onClick={resetForm}
                disabled={isLoading}
                className="px-4 py-2 rounded-lg bg-gray-700 hover:bg-gray-600 text-white font-medium transition-colors disabled:opacity-50"
              >
                Cancel
              </button>
              <button
                onClick={editingId ? handleUpdate : handleAdd}
                disabled={isLoading}
                className="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-medium transition-colors disabled:opacity-50"
              >
                {isLoading ? 'Saving...' : editingId ? 'Update' : 'Add'}
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="space-y-3">
        {folders.length === 0 ? (
          <p className="text-gray-400 text-center py-8">No folders configured yet.</p>
        ) : (
          folders.map((folder) => (
            <div
              key={folder.id}
              className="flex justify-between items-center bg-dark-bg rounded-lg p-4 border border-dark-border hover:border-blue-500 transition-colors"
            >
              <div className="flex-1 min-w-0">
                <h3 className="text-white font-semibold truncate">{folder.name}</h3>
                <p className="text-gray-400 text-sm truncate">{folder.path}</p>
              </div>
              <div className="flex gap-2 ml-4 flex-shrink-0">
                <button
                  onClick={() => handleEdit(folder)}
                  disabled={isLoading}
                  className="px-3 py-1 rounded bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium transition-colors disabled:opacity-50"
                >
                  Edit
                </button>
                <button
                  onClick={() => handleDelete(folder.id)}
                  disabled={isLoading}
                  className="px-3 py-1 rounded bg-red-600 hover:bg-red-700 text-white text-sm font-medium transition-colors disabled:opacity-50"
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
