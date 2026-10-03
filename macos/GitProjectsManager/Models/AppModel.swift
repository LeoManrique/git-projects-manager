import AppKit
import Foundation
import Observation

enum SidebarItem: Hashable {
    case all
    case kanban
    case folder(String)
}

enum FolderFormTarget: Identifiable {
    case add
    case edit(MonitoredFolder)

    var id: String {
        switch self {
        case .add: "add"
        case .edit(let folder): folder.id
        }
    }
}

/// Application state and scan orchestration, implementing the behavior rules
/// of FRONTEND.md over the gpm-core FFI.
@MainActor
@Observable
final class AppModel {
    private let core: GpmCore

    /// Kanban board state; shares the same core instance.
    let kanban: KanbanModel

    // Data
    private(set) var folders: [MonitoredFolder] = []
    private(set) var results: [String: ScanResult] = [:] {
        didSet { updateCheckingRepos() }
    }

    // Scan / operation state
    /// One entry per folder with a scan in flight: never two scans of one
    /// folder (FRONTEND.md §5.2).
    private var inFlightScans: [String: InFlightScan] = [:] {
        didSet { updateCheckingRepos() }
    }
    /// Scan All presses (and other visible full scans) still running.
    private var visibleFullScans = 0
    var isFullScanning: Bool { visibleFullScans > 0 }
    /// Repos a visible scan has not checked yet; their rows show a spinner.
    private var checkingRepos: Set<String> = []
    private(set) var pullingRepos: Set<String> = []
    private(set) var cleaningRepos: Set<String> = []
    private(set) var isBulkPulling = false
    private(set) var isBulkCleaning = false

    // UI state
    /// The shared error surface (§5.6). Every message it shows also goes to
    /// the log: the banner truncates to two lines and is replaced by the next
    /// message, so it cannot be the only record.
    var errorMessage: String? {
        didSet {
            if let errorMessage, errorMessage != oldValue { AppLog.warn("banner: \(errorMessage)") }
        }
    }
    var searchText = ""
    var selection: SidebarItem? = .all
    var folderForm: FolderFormTarget?

    // Settings
    private(set) var availableTerminals: [TerminalApp] = []
    private(set) var availableEditors: [EditorApp] = []
    private(set) var defaultTerminalId: String?
    private(set) var defaultEditorId: String?
    private(set) var gitCleanPatterns: [String] = []

    // Scan bookkeeping (FRONTEND.md §5.1)
    /// When the last full scan started: what "Last scan" shows.
    private(set) var lastFullScanStartedAt: Date?
    private var lastScanStartedAt: Date?
    private var hasInitialScan = false

    var defaultTerminal: TerminalApp? { availableTerminals.first { $0.id == defaultTerminalId } }
    var defaultEditor: EditorApp? { availableEditors.first { $0.id == defaultEditorId } }

    init() throws {
        core = try GpmCore()
        kanban = KanbanModel(core: core)
    }

    // MARK: - Startup

    func start() async {
        async let foldersLoad: Void = loadFolders()
        async let settingsLoad: Void = loadSettings()
        _ = await (foldersLoad, settingsLoad)
        triggerInitialScanIfNeeded()
    }

    func loadFolders() async {
        do {
            folders = try core.getMonitoredFolders()
            forgetRemovedFolders()
        } catch {
            // Degrade silently to the empty state (FRONTEND.md §3).
            AppLog.error("failed to load folders: \(Self.message(error))")
        }
    }

    /// Drop the results of folders that are no longer monitored (§5.2). A scan
    /// of one still in flight clears its own entry when it ends.
    private func forgetRemovedFolders() {
        let ids = Set(folders.map(\.id))
        // Assigned only on a real change: every write to an observed property
        // invalidates the views reading it.
        if !results.keys.allSatisfy(ids.contains) { results = results.filter { ids.contains($0.key) } }
    }

    func loadSettings() async {
        availableTerminals = core.getAvailableTerminals()
        availableEditors = core.getAvailableEditors()
        gitCleanPatterns = core.getGitCleanPatterns()
        do {
            let settings = try core.getAppSettings()
            defaultTerminalId = settings.defaultTerminal
            defaultEditorId = settings.defaultEditor
        } catch {
            AppLog.error("failed to load settings: \(Self.message(error))")
        }
    }

    func loadGitCleanPatterns() {
        gitCleanPatterns = core.getGitCleanPatterns()
    }

    /// One-time auto-scan the first time the folder list becomes non-empty
    /// in this session (FRONTEND.md §3).
    private func triggerInitialScanIfNeeded() {
        guard !hasInitialScan, !folders.isEmpty else { return }
        hasInitialScan = true
        Task { await scanAll() }
    }

    // MARK: - Scanning (FRONTEND.md §5)

    /// A folder's scan in flight, and whether it shows its progress.
    private struct InFlightScan {
        /// Ends with the scan's start (unix ms), or nil if it never finished.
        let task: Task<Int64?, Never>
        var isVisible: Bool
    }

    /// Scan `targets` concurrently and wait for all of them (§5.1–5.2). The
    /// single entry point behind every trigger: a folder already scanning is
    /// joined instead of scanned twice, and turns visible when this request
    /// is. A visible scan shows its progress; a silent one only its results.
    /// Returns when each target's scan started, nil for one that failed.
    @discardableResult
    private func requestScan(_ targets: [MonitoredFolder], visible: Bool) async -> [Int64?] {
        lastScanStartedAt = Date()
        var scans: [Task<Int64?, Never>] = []
        for folder in targets {
            if let inFlight = inFlightScans[folder.id] {
                if visible, !inFlight.isVisible { inFlightScans[folder.id]?.isVisible = true }
                scans.append(inFlight.task)
            } else {
                // Runs on the main actor once this loop yields, so its entry
                // is in place before the scan can end and remove it.
                let task = Task { await runScan(folder) }
                inFlightScans[folder.id] = InFlightScan(task: task, isVisible: visible)
                scans.append(task)
            }
        }
        var starts: [Int64?] = []
        for scan in scans { starts.append(await scan.value) }
        return starts
    }

    /// One folder's scan: every snapshot the core streams lands on screen as
    /// it arrives, the last one being the complete result. Returns when the
    /// scan started, or nil if it ended before it finished.
    private func runScan(_ folder: MonitoredFolder) async -> Int64? {
        let scan = core.startFolderScan(
            path: folder.path,
            onlyLocalChecks: folder.onlyLocalChecks,
            detectUninitialized: folder.detectUninitialized
        )
        var last: ScanResult?
        while let snapshot = await scan.next() {
            apply(snapshot, to: folder.id)
            last = snapshot
        }
        inFlightScans[folder.id] = nil
        // Ends early only when the core's scan thread died. The folder keeps
        // what it had (§5.1), so the log is the only place this shows up.
        guard let last, last.isComplete else {
            if self.folder(withId: folder.id) != nil { AppLog.error("scan of \(folder.path) ended before it finished") }
            return nil
        }
        return last.startedAtMs
    }

    /// Show `snapshot` as `folderId`'s result, unless the folder is gone or
    /// what is shown is already as new.
    private func apply(_ snapshot: ScanResult, to folderId: String) {
        guard folder(withId: folderId) != nil else { return }
        if let shown = results[folderId], shown.revision >= snapshot.revision { return }
        results[folderId] = snapshot
    }

    private func updateCheckingRepos() {
        let visible = inFlightScans.filter { $0.value.isVisible }.keys
        let pending = Set(visible.flatMap { results[$0]?.pending ?? [] })
        if pending != checkingRepos { checkingRepos = pending }
    }

    /// Whether `folderId` has a visible scan in flight.
    func isScanning(_ folderId: String) -> Bool {
        inFlightScans[folderId]?.isVisible == true
    }

    func isChecking(repoPath: String) -> Bool {
        checkingRepos.contains(repoPath)
    }

    /// Full scan of all folders, with global + per-folder progress indicators.
    /// Drives Scan All, the startup auto-scan and the window-focus rescan.
    /// On-demand, so it clears the shared error surface (§5.6).
    func scanAll() async {
        errorMessage = nil
        visibleFullScans += 1
        recordFullScan(startedAt: await requestScan(folders, visible: true))
        visibleFullScans -= 1
    }

    /// Move the "Last scan" clock to the start of a full scan that just
    /// ended: the earliest of its folders' scans, joined ones included, so the
    /// label never claims a folder is fresher than it is. A folder whose scan
    /// failed does not hold it back, and it never moves back.
    private func recordFullScan(startedAt starts: [Int64?]) {
        guard let earliest = starts.compactMap(\.self).min() else { return }
        let startedAt = Date(timeIntervalSince1970: Double(earliest) / 1000)
        if lastFullScanStartedAt.map({ startedAt > $0 }) ?? true { lastFullScanStartedAt = startedAt }
    }

    /// Scan a single folder (per-folder Scan control). On-demand, so it
    /// clears the shared error surface (§5.6).
    func scan(folder: MonitoredFolder) async {
        errorMessage = nil
        await requestScan([folder], visible: true)
    }

    // MARK: - The scan control (FRONTEND.md §5.1)

    /// The folder the current view is about, if it is a folder's detail view.
    private var selectedFolder: MonitoredFolder? {
        if case .folder(let id) = selection { return folder(withId: id) }
        return nil
    }

    /// The folders the current view is about: one in a folder's detail view,
    /// every folder anywhere else. The scan control acts on exactly these, and
    /// the local-checks notice describes exactly these, so the button and the
    /// notice can never disagree about what a scan covers.
    var foldersInView: [MonitoredFolder] {
        selectedFolder.map { [$0] } ?? folders
    }

    /// Whether the folders in view are being scanned — by their own scan or by
    /// a full one, which is why the folder case asks about the folder.
    var isScanningSelection: Bool {
        selectedFolder.map { isScanning($0.id) } ?? isFullScanning
    }

    var scanActionTitle: String {
        selectedFolder == nil ? "Scan All" : "Scan Folder"
    }

    var scanActionHelp: String {
        if let folder = selectedFolder { return "Rescan \(folder.name) (⌘R)" }
        return "Scan all monitored folders (⌘R)"
    }

    /// Scan what the current view shows. One entry point behind the toolbar
    /// button and ⌘R: "scan" always means "scan what I am looking at", so a
    /// folder's detail view rescans that folder and everything else runs the
    /// full scan.
    func scanSelection() async {
        if let folder = selectedFolder {
            await scan(folder: folder)
        } else {
            await scanAll()
        }
    }

    /// `repoPaths` grouped by the monitored folder that contains them. A repo
    /// no folder contains is left out: it belongs to no result on screen.
    private func reposByFolder(_ repoPaths: [String]) -> [(folder: MonitoredFolder, repos: [String])] {
        var groups: [String: (folder: MonitoredFolder, repos: [String])] = [:]
        for repoPath in repoPaths {
            // Longest match wins, so nested monitored folders attribute right.
            let best = folders
                .filter { Self.isInside(repoPath, $0.path) }
                .max { $0.path.count < $1.path.count }
            if let best { groups[best.id, default: (best, [])].repos.append(repoPath) }
        }
        return Array(groups.values)
    }

    private nonisolated static func isInside(_ repoPath: String, _ folderPath: String) -> Bool {
        var base = folderPath
        while base.hasSuffix("/") { base.removeLast() }
        return repoPath == base || repoPath.hasPrefix(base + "/")
    }

    /// Window regained focus: rescan all folders with the normal scan
    /// indicators, throttled to once per 20s since the last scan of any kind
    /// (§5.1). Skipped while any scan is in flight.
    func appDidBecomeActive() {
        kanban.appDidBecomeActive()
        guard hasInitialScan, !folders.isEmpty else { return }
        guard inFlightScans.isEmpty else { return }
        if let last = lastScanStartedAt, Date().timeIntervalSince(last) < 20 { return }
        Task { await scanAll() }
    }

    // MARK: - Folder CRUD (FRONTEND.md §4)

    /// Returns a user-facing error message, or nil on success.
    func saveFolder(
        target: FolderFormTarget,
        path: String,
        name: String,
        onlyLocalChecks: Bool,
        detectUninitialized: Bool
    ) async -> String? {
        do {
            switch target {
            case .add:
                _ = try core.addMonitoredFolder(
                    path: path,
                    name: name,
                    onlyLocalChecks: onlyLocalChecks,
                    detectUninitialized: detectUninitialized
                )
            case .edit(let folder):
                try core.updateMonitoredFolder(
                    id: folder.id,
                    path: path,
                    name: name,
                    onlyLocalChecks: onlyLocalChecks,
                    detectUninitialized: detectUninitialized
                )
            }
        } catch {
            // The core's message explains *why* the folder was refused (an
            // overlap with another monitored folder, say); the generic
            // fallback covers errors that carry no message of their own.
            let reason = Self.message(error)
            AppLog.error("folder save failed: \(reason)")
            if !reason.isEmpty { return reason }
            switch target {
            case .add: return "Failed to add folder"
            case .edit: return "Failed to update folder"
            }
        }
        await loadFolders()
        triggerInitialScanIfNeeded()
        return nil
    }

    func deleteFolder(_ folder: MonitoredFolder) {
        Task {
            do {
                try core.deleteMonitoredFolder(id: folder.id)
            } catch {
                AppLog.error("folder delete failed: \(Self.message(error))")
                errorMessage = "Failed to delete folder"
                return
            }
            if selection == .folder(folder.id) { selection = .all }
            await loadFolders()
        }
    }

    // MARK: - Repo operations (FRONTEND.md §5.5)

    /// Every action rechecks its repos, failed or not (a pull can fail after
    /// its fetch moved the counts), and keeps them flagged until the recheck
    /// lands, so the row spins from the click until it shows the new state.
    func pull(repoPath: String) async {
        pullingRepos.insert(repoPath)
        defer { pullingRepos.remove(repoPath) }
        do {
            _ = try await core.pullRepo(path: repoPath)
        } catch {
            errorMessage = "Failed to pull \(repoPath): \(Self.message(error))"
        }
        await recheck([repoPath])
    }

    func clean(repoPath: String) async {
        cleaningRepos.insert(repoPath)
        defer { cleaningRepos.remove(repoPath) }
        do {
            let result = try await core.cleanRepo(path: repoPath)
            if result.filesRemoved.isEmpty && result.directoriesRemoved.isEmpty {
                errorMessage = "No ignored files to clean in \(Self.repoName(repoPath))"
            }
        } catch {
            errorMessage = "Failed to clean \(repoPath): \(Self.message(error))"
        }
        await recheck([repoPath])
    }

    /// Read `repoPaths` again after an action changed them, each folder's in
    /// one call (§5.5). Never starts a scan, and never clears the message the
    /// action just set (§5.6). A folder the core has not scanned since its
    /// path was last set returns nothing, and keeps what it shows.
    private func recheck(_ repoPaths: [String]) async {
        await withTaskGroup(of: (String, ScanResult?).self) { [core] group in
            for (folder, repos) in reposByFolder(repoPaths) {
                group.addTask {
                    do {
                        let snapshot = try await core.recheckRepos(
                            folder: folder.path,
                            repos: repos,
                            onlyLocalChecks: folder.onlyLocalChecks
                        )
                        return (folder.id, snapshot)
                    } catch {
                        AppLog.error("recheck in \(folder.path) failed: \(Self.message(error))")
                        return (folder.id, nil)
                    }
                }
            }
            for await (folderId, snapshot) in group {
                if let snapshot { apply(snapshot, to: folderId) }
            }
        }
    }

    func pullAll(_ repos: [RepoStatus]) async {
        guard !isBulkPulling else { return }
        isBulkPulling = true
        let paths = repos.map(\.path)
        pullingRepos.formUnion(paths)

        let failures = await runOnEach(paths) { [core] path in
            _ = try await core.pullRepo(path: path)
        }
        if let first = failures.first {
            errorMessage = "Failed to pull \(failures.count) repo(s): \(first)"
        }
        await recheck(paths)

        pullingRepos.subtract(paths)
        isBulkPulling = false
    }

    func cleanAll(_ repos: [RepoStatus]) async {
        guard !isBulkCleaning else { return }
        isBulkCleaning = true
        let paths = repos.map(\.path)
        cleaningRepos.formUnion(paths)

        let failures = await runOnEach(paths) { [core] path in
            _ = try await core.cleanRepo(path: path)
        }
        if let first = failures.first {
            errorMessage = "Failed to clean \(failures.count) repo(s): \(first)"
        }
        await recheck(paths)

        cleaningRepos.subtract(paths)
        isBulkCleaning = false
    }

    /// Run `operation` on every path concurrently and collect the failures as
    /// `"{repo}: {reason}"`. The reason used to be discarded by a `try?`, so a
    /// bulk action could only ever report a count.
    private nonisolated func runOnEach(
        _ paths: [String],
        _ operation: @escaping @Sendable (String) async throws -> Void
    ) async -> [String] {
        await withTaskGroup(of: String?.self) { group in
            for path in paths {
                group.addTask {
                    do {
                        try await operation(path)
                        return nil
                    } catch {
                        return "\(Self.repoName(path)): \(Self.message(error))"
                    }
                }
            }
            var failures: [String] = []
            for await failure in group {
                if let failure { failures.append(failure) }
            }
            return failures
        }
    }

    // MARK: - Open actions (FRONTEND.md §5.5)

    func openInEditor(_ path: String) {
        guard let editor = defaultEditor else { return }
        do {
            try core.openInEditor(path: path, editorId: editor.id)
        } catch {
            errorMessage = "Failed to open editor: \(Self.message(error))"
        }
    }

    func openInTerminal(_ path: String) {
        guard let terminal = defaultTerminal else { return }
        do {
            try core.openInTerminal(path: path, terminalId: terminal.id)
        } catch {
            errorMessage = "Failed to open terminal: \(Self.message(error))"
        }
    }

    func openInLmsGithub(_ path: String) {
        do {
            try core.openInLmsGithub(path: path)
        } catch {
            errorMessage = "Failed to open LMS Github: \(Self.message(error))"
        }
    }

    func revealInFinder(_ path: String) {
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
    }

    // MARK: - Diagnostics log (FRONTEND.md §6.4)

    /// Where the log files live, for the Logs settings tab to show.
    func logsFolderPath() -> String? {
        do {
            return try logsFolder()
        } catch {
            AppLog.error("failed to find the logs folder: \(Self.message(error))")
            return nil
        }
    }

    /// Returns a user-facing error message, or nil on success.
    func showLogsFolder() -> String? {
        do {
            try openLogsFolder()
            return nil
        } catch {
            AppLog.error("failed to open the logs folder: \(Self.message(error))")
            return "Failed to open the logs folder"
        }
    }

    func copyPath(_ path: String) {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(path, forType: .string)
    }

    // MARK: - Settings (FRONTEND.md §6)

    /// Returns a user-facing error message, or nil on success.
    func setDefaultTerminal(_ id: String?) -> String? {
        do {
            try core.setDefaultTerminal(terminalId: id)
            defaultTerminalId = id
            return nil
        } catch {
            AppLog.error("failed to save the default terminal: \(Self.message(error))")
            return "Failed to save setting"
        }
    }

    func setDefaultEditor(_ id: String?) -> String? {
        do {
            try core.setDefaultEditor(editorId: id)
            defaultEditorId = id
            return nil
        } catch {
            AppLog.error("failed to save the default editor: \(Self.message(error))")
            return "Failed to save setting"
        }
    }

    func addGitCleanPattern(_ raw: String) -> String? {
        let pattern = raw.trimmingCharacters(in: .whitespaces)
        guard !pattern.isEmpty else { return nil }
        guard !gitCleanPatterns.contains(pattern) else { return "Pattern already exists" }
        return persistGitCleanPatterns(gitCleanPatterns + [pattern])
    }

    func removeGitCleanPattern(_ pattern: String) -> String? {
        persistGitCleanPatterns(gitCleanPatterns.filter { $0 != pattern })
    }

    private func persistGitCleanPatterns(_ patterns: [String]) -> String? {
        do {
            try core.setGitCleanPatterns(patterns: patterns)
            gitCleanPatterns = patterns
            return nil
        } catch {
            AppLog.error("failed to save git clean settings: \(Self.message(error))")
            return "Failed to save settings"
        }
    }

    // MARK: - Derived helpers

    func folder(withId id: String) -> MonitoredFolder? {
        folders.first { $0.id == id }
    }

    /// Repos needing attention (changed + unpushed + unpulled + errors).
    func attentionCount(for folderId: String) -> Int {
        guard let result = results[folderId] else { return 0 }
        return result.withChanges.count + result.withUnpushed.count
            + result.withUnpulled.count + result.errors.count
    }

    /// A folder's sections in display order: search-filtered, empty ones
    /// dropped, and Checking only while a visible scan is checking them
    /// (FRONTEND.md §5.3).
    func visibleSections(of folderId: String) -> [(category: RepoCategory, repos: [RepoStatus])] {
        guard let result = results[folderId] else { return [] }
        let isScanning = isScanning(folderId)
        return RepoCategory.allCases.compactMap { category in
            guard isScanning || !category.onlyWhileScanning else { return nil }
            let repos = filtered(category.repos(in: result))
            return repos.isEmpty ? nil : (category, repos)
        }
    }

    /// Search filter (FRONTEND.md §5.4): case-insensitive substring on repo
    /// name or full path.
    func filtered(_ repos: [RepoStatus]) -> [RepoStatus] {
        let query = searchText.trimmingCharacters(in: .whitespaces).lowercased()
        guard !query.isEmpty else { return repos }
        return repos.filter { repo in
            Self.repoName(repo.path).lowercased().contains(query)
                || repo.path.lowercased().contains(query)
        }
    }

    func isBusy(repoPath: String) -> Bool {
        pullingRepos.contains(repoPath) || cleaningRepos.contains(repoPath)
    }

    nonisolated static func repoName(_ path: String) -> String {
        URL(fileURLWithPath: path).lastPathComponent
    }

    nonisolated static func message(_ error: Error) -> String {
        if case let GpmError.Failure(message) = error { return message }
        return error.localizedDescription
    }
}
