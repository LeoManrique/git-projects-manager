import React, { useState } from 'react';
import { api } from '../lib/api';
import { MonitoredFolder } from '../types';

interface FolderManagerProps {
  folders: MonitoredFolder[];
  onFoldersChange: () => void;
}

export const FolderManager: React.FC<FolderManagerProps> = ({ folders, onFoldersChange }) => {
  const [isAdding, setIsAdding] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [newFolder, setNewFolder] = useState({ path: '', name: '' });
  const [editFolder, setEditFolder] = useState({ path: '', name: '' });
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleAddFolder = async () => {
    if (!newFolder.path || !newFolder.name) {
      setError('Please provide both path and name');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      await api.addMonitoredFolder(newFolder.path, newFolder.name);
      setNewFolder({ path: '', name: '' });
      setIsAdding(false);
      onFoldersChange();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add folder');
    } finally {
      setLoading(false);
    }
  };

  const handleUpdateFolder = async (id: string) => {
    if (!editFolder.path || !editFolder.name) {
      setError('Please provide both path and name');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      await api.updateMonitoredFolder(id, editFolder.path, editFolder.name);
      setEditingId(null);
      setEditFolder({ path: '', name: '' });
      onFoldersChange();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update folder');
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteFolder = async (id: string) => {
    if (!confirm('Are you sure you want to delete this folder?')) {
      return;
    }

    setLoading(true);
    setError(null);

    try {
      await api.deleteMonitoredFolder(id);
      onFoldersChange();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete folder');
    } finally {
      setLoading(false);
    }
  };

  const handleBrowseFolder = async () => {
    try {
      const selectedPath = await api.browseFolder();
      if (selectedPath) {
        if (isAdding) {
          setNewFolder({ ...newFolder, path: selectedPath });
        } else if (editingId) {
          setEditFolder({ ...editFolder, path: selectedPath });
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to browse folder');
    }
  };

  const startEdit = (folder: MonitoredFolder) => {
    setEditingId(folder.id);
    setEditFolder({ path: folder.path, name: folder.name });
    setIsAdding(false);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditFolder({ path: '', name: '' });
    setError(null);
  };

  const startAdd = () => {
    setIsAdding(true);
    setEditingId(null);
    setNewFolder({ path: '', name: '' });
    setError(null);
  };

  const cancelAdd = () => {
    setIsAdding(false);
    setNewFolder({ path: '', name: '' });
    setError(null);
  };

  return (
    <div className="folder-manager">
      <div className="header">
        <h2>Monitored Folders</h2>
        {!isAdding && (
          <button
            onClick={startAdd}
            disabled={loading}
            className="btn btn-primary"
          >
            Add Folder
          </button>
        )}
      </div>

      {error && (
        <div className="error-message">
          {error}
        </div>
      )}

      {isAdding && (
        <div className="folder-form">
          <div className="form-group">
            <label>Path:</label>
            <div className="input-with-button">
              <input
                type="text"
                value={newFolder.path}
                onChange={(e) => setNewFolder({ ...newFolder, path: e.target.value })}
                placeholder="Enter folder path"
                disabled={loading}
              />
              <button
                onClick={handleBrowseFolder}
                disabled={loading}
                className="btn btn-secondary"
              >
                Browse
              </button>
            </div>
          </div>
          <div className="form-group">
            <label>Name:</label>
            <input
              type="text"
              value={newFolder.name}
              onChange={(e) => setNewFolder({ ...newFolder, name: e.target.value })}
              placeholder="Enter display name"
              disabled={loading}
            />
          </div>
          <div className="form-actions">
            <button
              onClick={handleAddFolder}
              disabled={loading}
              className="btn btn-success"
            >
              {loading ? 'Adding...' : 'Add'}
            </button>
            <button
              onClick={cancelAdd}
              disabled={loading}
              className="btn btn-cancel"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      <div className="folder-list">
        {folders.length === 0 ? (
          <div className="empty-state">
            <p>No folders monitored yet.</p>
            <p>Click "Add Folder" to start monitoring Git repositories.</p>
          </div>
        ) : (
          folders.map((folder) => (
            <div key={folder.id} className="folder-item">
              {editingId === folder.id ? (
                <div className="folder-edit-form">
                  <div className="form-group">
                    <label>Path:</label>
                    <div className="input-with-button">
                      <input
                        type="text"
                        value={editFolder.path}
                        onChange={(e) => setEditFolder({ ...editFolder, path: e.target.value })}
                        disabled={loading}
                      />
                      <button
                        onClick={handleBrowseFolder}
                        disabled={loading}
                        className="btn btn-secondary"
                      >
                        Browse
                      </button>
                    </div>
                  </div>
                  <div className="form-group">
                    <label>Name:</label>
                    <input
                      type="text"
                      value={editFolder.name}
                      onChange={(e) => setEditFolder({ ...editFolder, name: e.target.value })}
                      disabled={loading}
                    />
                  </div>
                  <div className="form-actions">
                    <button
                      onClick={() => handleUpdateFolder(folder.id)}
                      disabled={loading}
                      className="btn btn-success"
                    >
                      {loading ? 'Saving...' : 'Save'}
                    </button>
                    <button
                      onClick={cancelEdit}
                      disabled={loading}
                      className="btn btn-cancel"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <div className="folder-display">
                  <div className="folder-info">
                    <h3>{folder.name}</h3>
                    <p className="folder-path">{folder.path}</p>
                  </div>
                  <div className="folder-actions">
                    <button
                      onClick={() => startEdit(folder)}
                      disabled={loading}
                      className="btn btn-secondary"
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => handleDeleteFolder(folder.id)}
                      disabled={loading}
                      className="btn btn-danger"
                    >
                      Delete
                    </button>
                  </div>
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
};