import { useState } from 'react';
import { api } from '../../lib/api';
import { MonitoredFolder } from '../../types';
import { FolderForm } from './FolderForm';

interface FolderManagerProps {
  folders: MonitoredFolder[];
  onRefresh: () => Promise<void>;
}

export default function FolderManager({ folders, onRefresh }: FolderManagerProps) {
  const [showAddForm, setShowAddForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  const resetForm = () => {
    setEditingId(null);
    setShowAddForm(false);
    setError('');
  };

  const executeOperation = async (operation: () => Promise<unknown>, errorMsg: string) => {
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

  const handleAdd = async (path: string, name: string) => {
    await executeOperation(() => api.addMonitoredFolder(path, name), 'Failed to add folder');
  };

  const handleUpdate = async (path: string, name: string) => {
    if (!editingId) return;
    await executeOperation(() => api.updateMonitoredFolder(editingId, path, name), 'Failed to update folder');
  };

  const handleDelete = (id: string) => {
    if (!window.confirm('Are you sure you want to delete this folder?')) return;
    executeOperation(() => api.deleteMonitoredFolder(id), 'Failed to delete folder');
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
    setShowAddForm(true);
    setError('');
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex-shrink-0 flex justify-between items-center mb-2">
        <h2 className="text-sm font-medium text-text-primary">Monitored Folders</h2>
        <button
          onClick={toggleForm}
          className="bg-accent-blue/90 hover:bg-accent-blue text-white text-xs font-medium py-1 px-2.5 rounded transition-colors"
        >
          {showAddForm && !editingId ? 'Cancel' : 'Add Folder'}
        </button>
      </div>

      {error && (
        <div className="flex-shrink-0 bg-accent-red/10 border border-accent-red/20 text-accent-red px-2.5 py-1.5 rounded text-xs mb-2">
          {error}
        </div>
      )}

      {/* Add new folder form (only when not editing existing) */}
      {showAddForm && !editingId && (
        <div className="flex-shrink-0">
          <FolderForm
            onSubmit={handleAdd}
            onCancel={resetForm}
            submitLabel="Add"
            isLoading={isLoading}
            variant="standalone"
          />
        </div>
      )}

      <div className="flex-1 overflow-auto space-y-1.5">
        {folders.length === 0 ? (
          <p className="text-text-muted text-xs text-center py-6">No folders configured yet.</p>
        ) : (
          folders.map((folder) => {
            const isEditing = editingId === folder.id;

            return (
              <div
                key={folder.id}
                className="bg-dark-surface rounded border border-dark-border overflow-hidden"
              >
                {/* Folder header */}
                <div className="flex justify-between items-center px-2.5 py-2 hover:bg-dark-elevated/50 transition-colors">
                  <div className="flex-1 min-w-0">
                    <h3 className="text-text-primary text-xs font-medium truncate">{folder.name}</h3>
                    <p className="text-text-muted text-xs truncate font-mono mt-0.5">{folder.path}</p>
                  </div>
                  <div className="flex gap-1.5 ml-2 flex-shrink-0">
                    <button
                      onClick={() => isEditing ? resetForm() : handleEdit(folder)}
                      disabled={isLoading}
                      className="px-2 py-0.5 rounded-sm bg-dark-borderStrong hover:bg-dark-elevated text-text-secondary hover:text-text-primary text-xs font-medium transition-colors disabled:opacity-40"
                    >
                      {isEditing ? 'Cancel' : 'Edit'}
                    </button>
                    <button
                      onClick={() => handleDelete(folder.id)}
                      disabled={isLoading || isEditing}
                      className="px-2 py-0.5 rounded-sm bg-accent-red/15 hover:bg-accent-red/25 text-accent-red text-xs font-medium transition-colors disabled:opacity-40"
                    >
                      Delete
                    </button>
                  </div>
                </div>

                {/* Inline edit form */}
                {isEditing && (
                  <FolderForm
                    initialPath={folder.path}
                    initialName={folder.name}
                    onSubmit={handleUpdate}
                    onCancel={resetForm}
                    submitLabel="Save"
                    isLoading={isLoading}
                    variant="inline"
                  />
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
