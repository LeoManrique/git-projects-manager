import { useState, useEffect, useCallback } from 'react';
import FolderManager from './components/FolderManager';
import ScanResults, { ScanResultsState } from './components/ScanResults';
import { GetMonitoredFolders } from '../wailsjs/go/app/App';
import { WindowMinimise, WindowToggleMaximise, Quit } from '../wailsjs/runtime/runtime';
import './App.css';
import logo from './assets/images/logo-universal.png';

export interface MonitoredFolder {
    id: string;
    path: string;
    name: string;
}

// Detect platform for conditional styling
const isMac = navigator.userAgent.toUpperCase().indexOf('MAC') >= 0;

function App() {
    const [folders, setFolders] = useState<MonitoredFolder[]>([]);
    const [activeTab, setActiveTab] = useState<'manage' | 'scan'>('scan');
    const [scanState, setScanState] = useState<ScanResultsState>({
        results: {},
        expandedFolders: new Set(),
    });

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
        <div className="h-screen flex flex-col bg-dark-bg text-text-primary overflow-hidden">
            {/* Title Bar */}
            <header
                className="flex-shrink-0 bg-dark-surface border-b border-dark-border h-9 flex items-center select-none wails-drag"
            >
                {/* Left section: Logo + App name (with macOS traffic light padding) */}
                <div className={`flex items-center h-full ${isMac ? 'pl-[70px]' : 'pl-3'}`}>
                    <img src={logo} alt="logo" className="w-4 h-4 mr-2 pointer-events-none" />
                    <span className="text-sm font-medium text-text-primary">Git Projects Manager</span>
                </div>

                {/* Spacer */}
                <div className="flex-1" />

                {/* Window controls (Windows/Linux only) */}
                {!isMac && (
                    <div className="flex h-full wails-no-drag">
                        <button
                            onClick={() => WindowMinimise()}
                            className="w-12 h-full flex items-center justify-center text-text-secondary hover:bg-dark-elevated transition-colors"
                            title="Minimize"
                        >
                            <svg width="10" height="1" viewBox="0 0 10 1" fill="currentColor">
                                <rect width="10" height="1" />
                            </svg>
                        </button>
                        <button
                            onClick={() => WindowToggleMaximise()}
                            className="w-12 h-full flex items-center justify-center text-text-secondary hover:bg-dark-elevated transition-colors"
                            title="Maximize"
                        >
                            <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor">
                                <rect x="0.5" y="0.5" width="9" height="9" strokeWidth="1" />
                            </svg>
                        </button>
                        <button
                            onClick={() => Quit()}
                            className="w-12 h-full flex items-center justify-center text-text-secondary hover:bg-accent-red hover:text-white transition-colors"
                            title="Close"
                        >
                            <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
                                <path d="M1 1L9 9M9 1L1 9" stroke="currentColor" strokeWidth="1.2" />
                            </svg>
                        </button>
                    </div>
                )}
            </header>

            {/* Tab Navigation */}
            <nav className="flex-shrink-0 bg-dark-surface border-b border-dark-border px-3">
                <div className="flex">
                    <button
                        onClick={() => setActiveTab('scan')}
                        className={`px-3 py-2 text-sm font-medium transition-colors relative ${
                            activeTab === 'scan'
                                ? 'text-text-primary'
                                : 'text-text-secondary hover:text-text-primary'
                        }`}
                    >
                        Scan Results
                        {activeTab === 'scan' && (
                            <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-blue" />
                        )}
                    </button>
                    <button
                        onClick={() => setActiveTab('manage')}
                        className={`px-3 py-2 text-sm font-medium transition-colors relative ${
                            activeTab === 'manage'
                                ? 'text-text-primary'
                                : 'text-text-secondary hover:text-text-primary'
                        }`}
                    >
                        Manage Folders
                        {activeTab === 'manage' && (
                            <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-blue" />
                        )}
                    </button>
                </div>
            </nav>

            {/* Main Content */}
            <main className="flex-1 overflow-auto p-4">
                {activeTab === 'manage' && (
                    <FolderManager folders={folders} onRefresh={loadFolders} />
                )}
                {activeTab === 'scan' && (
                    <ScanResults
                        folders={folders}
                        scanState={scanState}
                        onScanStateChange={setScanState}
                    />
                )}
            </main>
        </div>
    );
}

export default App;
