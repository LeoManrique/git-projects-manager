import { useState, useEffect, useCallback } from 'react';
import FolderManager from './components/FolderManager';
import ScanResults from './components/ScanResults';
import { GetMonitoredFolders } from '../wailsjs/go/main/App';
import './App.css';
import logo from './assets/images/logo-universal.png';

export interface MonitoredFolder {
    id: string;
    path: string;
    name: string;
}

function App() {
    const [folders, setFolders] = useState<MonitoredFolder[]>([]);
    const [activeTab, setActiveTab] = useState<'manage' | 'scan'>('scan');

    const loadFolders = useCallback(async () => {
        try {
            const loaded = await GetMonitoredFolders();
            setFolders(loaded || []);
        } catch (err) {
            console.error('Failed to load folders:', err);
        }
    }, []);

    useEffect(() => {
        loadFolders();
    }, [loadFolders]);

    return (
        <div className="min-h-screen bg-dark-bg text-white">
            {/* Header */}
            <header className="bg-dark-surface border-b border-dark-border shadow-lg">
                <div className="max-w-6xl mx-auto px-6 py-6">
                    <div className="flex items-center gap-4">
                        <img src={logo} alt="logo" className="w-10 h-10" />
                        <div>
                            <h1 className="text-3xl font-bold">Git Projects Manager</h1>
                            <p className="text-gray-400 text-sm">Monitor and manage multiple Git repositories</p>
                        </div>
                    </div>
                </div>
            </header>

            {/* Tab Navigation */}
            <nav className="bg-dark-surface border-b border-dark-border sticky top-0 z-10">
                <div className="max-w-6xl mx-auto px-6">
                    <div className="flex gap-8">
                        <button
                            onClick={() => setActiveTab('scan')}
                            className={`py-4 px-2 font-semibold border-b-2 transition-colors ${
                                activeTab === 'scan'
                                    ? 'border-blue-600 text-blue-400'
                                    : 'border-transparent text-gray-400 hover:text-gray-300'
                            }`}
                        >
                            Scan Results
                        </button>
                        <button
                            onClick={() => setActiveTab('manage')}
                            className={`py-4 px-2 font-semibold border-b-2 transition-colors ${
                                activeTab === 'manage'
                                    ? 'border-blue-600 text-blue-400'
                                    : 'border-transparent text-gray-400 hover:text-gray-300'
                            }`}
                        >
                            Manage Folders
                        </button>
                    </div>
                </div>
            </nav>

            {/* Main Content */}
            <main className="max-w-6xl mx-auto px-6 py-8">
                {activeTab === 'manage' && (
                    <FolderManager folders={folders} onRefresh={loadFolders} />
                )}
                {activeTab === 'scan' && (
                    <ScanResults folders={folders} />
                )}
            </main>

            {/* Footer */}
            <footer className="bg-dark-surface border-t border-dark-border mt-12">
                <div className="max-w-6xl mx-auto px-6 py-4 text-center text-gray-400 text-sm">
                    <p>Git Projects Manager © 2025</p>
                </div>
            </footer>
        </div>
    );
}

export default App;
