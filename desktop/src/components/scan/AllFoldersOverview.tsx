import { MonitoredFolder, ScanResult } from '../../types';
import { UseScannerReturn } from '../../hooks/useScanner';
import { StatusBadge } from './StatusBadge';
import { RepoSection } from './RepoSection';
import { RepoActionHandlers } from './RepoRow';
import { SECTIONS, visibleSections } from './sections';
import { FolderIcon, OpenDetailIcon, RefreshIcon } from '../icons';

interface AllFoldersOverviewProps {
  folders: MonitoredFolder[];
  scanner: UseScannerReturn;
  searchQuery: string;
  handlers: RepoActionHandlers;
  onOpenFolder: (folderId: string) => void;
  onAddFolder: () => void;
}

/**
 * Dashboard listing every monitored folder with all of its repo sections
 * expanded inline, mirroring the macOS All Folders view (FRONTEND.md §5.3):
 * pending commit/pull work is visible without opening a folder, with Clean
 * summarized in the header and listed last.
 */
export function AllFoldersOverview({
  folders,
  scanner,
  searchQuery,
  handlers,
  onOpenFolder,
  onAddFolder,
}: AllFoldersOverviewProps) {
  if (folders.length === 0) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-3 text-center">
        <FolderIcon className="w-8 h-8 text-text-muted" />
        <div>
          <p className="text-sm text-text-primary font-medium">No folders configured</p>
          <p className="text-xs text-text-muted mt-1">
            Add a folder to start monitoring its git repositories.
          </p>
        </div>
        <button
          onClick={onAddFolder}
          className="mt-1 px-3 py-1.5 rounded-md bg-accent-blue hover:bg-accent-blueHover text-white text-xs font-medium transition-colors"
        >
          Add Folder
        </button>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto">
      {folders.map((folder) => (
        <FolderOverviewGroup
          key={folder.id}
          folder={folder}
          scanner={scanner}
          searchQuery={searchQuery}
          handlers={handlers}
          onOpenFolder={onOpenFolder}
        />
      ))}
    </div>
  );
}

interface FolderOverviewGroupProps {
  folder: MonitoredFolder;
  scanner: UseScannerReturn;
  searchQuery: string;
  handlers: RepoActionHandlers;
  onOpenFolder: (folderId: string) => void;
}

function FolderOverviewGroup({
  folder,
  scanner,
  searchQuery,
  handlers,
  onOpenFolder,
}: FolderOverviewGroupProps) {
  const result = scanner.results[folder.id];
  const isScanning = scanner.scanningFolders.has(folder.id);

  return (
    <section className="pb-4">
      <FolderSummaryHeader
        folder={folder}
        result={result}
        isScanning={isScanning}
        onScan={() => scanner.scanFolder(folder)}
        onOpen={() => onOpenFolder(folder.id)}
      />

      <div className="px-2 pt-2 space-y-3">
        {result ? (
          <FolderOverviewBody
            result={result}
            searchQuery={searchQuery}
            scanner={scanner}
            handlers={handlers}
          />
        ) : isScanning ? (
          <div className="flex items-center gap-2 px-2 py-1 text-xs text-text-muted">
            <span className="w-3.5 h-3.5 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
            Scanning…
          </div>
        ) : (
          <button
            onClick={() => scanner.scanFolder(folder)}
            className="px-2 py-1 text-xs text-accent-blue hover:underline"
          >
            Scan Folder
          </button>
        )}
      </div>
    </section>
  );
}

function FolderOverviewBody({
  result,
  searchQuery,
  scanner,
  handlers,
}: {
  result: ScanResult;
  searchQuery: string;
  scanner: UseScannerReturn;
  handlers: RepoActionHandlers;
}) {
  const groups = visibleSections(SECTIONS, result, searchQuery);

  if (groups.length === 0) {
    return (
      <p className="px-2 py-1 text-xs text-text-muted">
        {searchQuery.trim() ? 'No matching repositories' : 'No repositories found'}
      </p>
    );
  }

  return (
    <>
      {groups.map(([spec, repos]) => (
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
    </>
  );
}

interface FolderSummaryHeaderProps {
  folder: MonitoredFolder;
  result: ScanResult | undefined;
  isScanning: boolean;
  onScan: () => void;
  onOpen: () => void;
}

/**
 * Sticky per-folder header: name, path, `{total} repos` + clean badge
 * summary (counts always unfiltered, §5.4), and scan / open-detail controls.
 */
function FolderSummaryHeader({ folder, result, isScanning, onScan, onOpen }: FolderSummaryHeaderProps) {
  return (
    <div className="sticky top-0 z-10 flex items-center gap-3 px-4 py-2 bg-dark-bg/95 backdrop-blur-sm border-b border-dark-border/60">
      <div className="flex-1 min-w-0">
        <h3 className="text-sm font-semibold text-text-primary truncate">{folder.name}</h3>
        <p className="text-[11px] leading-4 font-mono text-text-muted truncate">{folder.path}</p>
      </div>

      {isScanning ? (
        <span className="flex items-center gap-1.5 text-xs text-text-muted">
          <span className="w-3 h-3 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
          Scanning…
        </span>
      ) : result ? (
        <span className="flex items-center gap-2">
          <span className="text-xs text-text-secondary">{result.totalRepositories} repos</span>
          <StatusBadge count={result.clean.length} label="clean" color="green" />
        </span>
      ) : (
        <span className="text-xs text-text-muted">Not scanned</span>
      )}

      <span className="flex items-center gap-0.5">
        <button
          onClick={onScan}
          disabled={isScanning}
          className="p-1.5 rounded hover:bg-dark-border text-text-secondary hover:text-text-primary transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          title={`Scan ${folder.name}`}
        >
          <RefreshIcon />
        </button>
        <button
          onClick={onOpen}
          className="p-1.5 rounded hover:bg-dark-border text-text-secondary hover:text-text-primary transition-colors"
          title={`Open ${folder.name}`}
        >
          <OpenDetailIcon />
        </button>
      </span>
    </div>
  );
}
