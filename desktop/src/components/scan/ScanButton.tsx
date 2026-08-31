import { MonitoredFolder } from '../../types';
import { UseScannerReturn } from '../../hooks/useScanner';
import { RefreshIcon } from '../icons';

interface ScanButtonProps {
  /** The folder in view, or `undefined` in the All Folders overview. */
  folder?: MonitoredFolder;
  hasFolders: boolean;
  scanner: UseScannerReturn;
}

/**
 * The single scan control in the header (FRONTEND.md §5.1).
 *
 * What it scans follows the view: the overview scans every monitored folder, a
 * folder's detail scans that folder. One button rather than two, because "scan"
 * always means "scan what I am looking at" — a separate per-view control asked
 * the user to tell apart two buttons with the same intent, and the header's
 * Scan All scanned every folder even while a single folder was open.
 */
export function ScanButton({ folder, hasFolders, scanner }: ScanButtonProps) {
  const isScanning = folder
    ? scanner.scanningFolders.has(folder.id)
    : scanner.isFullScanning;

  return (
    <button
      onClick={() => (folder ? scanner.scanFolder(folder) : scanner.scanAll())}
      disabled={!hasFolders || isScanning}
      title={folder ? `Rescan ${folder.name}` : 'Scan all monitored folders'}
      className="flex-shrink-0 flex items-center gap-1.5 bg-accent-blue hover:bg-accent-blueHover text-white text-xs font-medium py-1.5 px-3 rounded-md transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
    >
      {isScanning ? (
        <>
          <span className="w-3 h-3 border-2 border-white/70 border-t-transparent rounded-full animate-spin" />
          Scanning…
        </>
      ) : (
        <>
          <RefreshIcon className="w-3 h-3" />
          {folder ? 'Scan Folder' : 'Scan All'}
        </>
      )}
    </button>
  );
}
