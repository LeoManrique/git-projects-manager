import { MonitoredFolder } from '../../types';
import { UseScannerReturn } from '../../hooks/useScanner';
import { RepoSection } from './RepoSection';
import { RepoActionHandlers } from './RepoRow';
import { SECTIONS, visibleSections } from './sections';

interface FolderDetailProps {
  folder: MonitoredFolder;
  scanner: UseScannerReturn;
  searchQuery: string;
  handlers: RepoActionHandlers;
}

/**
 * Scan results for a single monitored folder: all six category sections in
 * fixed order, including Clean with its bulk action, plus the execution-time
 * footer (FRONTEND.md §5.3).
 */
export function FolderDetail({ folder, scanner, searchQuery, handlers }: FolderDetailProps) {
  const result = scanner.results[folder.id];
  const isScanning = scanner.scanningFolders.has(folder.id);

  if (!result) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-3 text-center">
        {isScanning ? (
          <>
            <span className="w-5 h-5 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
            <p className="text-xs text-text-muted">Scanning…</p>
          </>
        ) : (
          <>
            <p className="text-sm text-text-primary font-medium">Not scanned</p>
            <p className="text-xs text-text-muted">
              Scan this folder to see the status of its repositories.
            </p>
            <button
              onClick={() => scanner.scanFolder(folder)}
              className="mt-1 px-3 py-1.5 rounded-md bg-accent-blue hover:bg-accent-blueHover text-white text-xs font-medium transition-colors"
            >
              Scan Folder
            </button>
          </>
        )}
      </div>
    );
  }

  const sections = visibleSections(SECTIONS, result, searchQuery);

  if (sections.length === 0 && searchQuery.trim()) {
    return (
      <div className="h-full flex items-center justify-center">
        <p className="text-xs text-text-muted">No results for “{searchQuery.trim()}”</p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <div className="flex-1 overflow-y-auto px-2 py-3 space-y-4">
        {sections.map(([spec, repos]) => (
          <RepoSection
            key={spec.key}
            spec={spec}
            repos={repos}
            handlers={handlers}
            onPullAll={scanner.pullAll}
            onCleanAll={scanner.cleanAll}
            isBulkPulling={scanner.isBulkPulling}
            isBulkCleaning={scanner.isBulkCleaning}
          />
        ))}
      </div>
      <div className="flex-shrink-0 px-4 py-2 border-t border-dark-borderSubtle text-[11px] text-text-muted">
        Completed in {(result.executionTime ?? 0).toFixed(2)}s
      </div>
    </div>
  );
}
