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
    private(set) var results: [String: ScanResult] = [:]

    // Scan / operation state
    private(set) var scanningFolders: Set<String> = []
    private(set) var isFullScanning = false
    private(set) var pullingRepos: Set<String> = []
    private(set) var cleaningRepos: Set<String> = []
    private(set) var isBulkPulling = false
    private(set) var isBulkCleaning = false

    // UI state
    var errorMessage: String?
    var searchText = ""
    var selection: SidebarItem? = .all
    var folderForm: FolderFormTarget?

    // Settings
    private(set) var availableTerminals: [TerminalApp] = []
    private(set) var availableEditors: [EditorApp] = []
    private(set) var defaultTerminalId: String?
    private(set) var defaultEditorId: String?
    private(set) var gitCleanPatterns: [String] = []

    // Scan bookkeeping (FRONTEND.md §5.1–5.2)
    private var scanVersion = 0
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
        } catch {
            // Degrade silently to the empty state (FRONTEND.md §3).
            NSLog("failed to load folders: \(Self.message(error))")
        }
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
            NSLog("failed to load settings: \(Self.message(error))")
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

    /// Scan the given folders concurrently, merging each result as it lands
    /// (§5.1). The single implementation behind every scan mode.
    ///
    /// - `isFullScan` bumps the supersession version and drives the global
    ///   indicator.
    /// - `clearError` is false for the rescans that follow a pull or clean, so
    ///   the action's own message survives the scan it triggers; per §5.6 the
    ///   error surface clears when the next *on-demand* scan starts.
    private func scan(
        folders targets: [MonitoredFolder],
        isFullScan: Bool,
        clearError: Bool
    ) async {
        guard !targets.isEmpty else { return }

        if isFullScan {
            scanVersion += 1
            isFullScanning = true
        }
        let version = scanVersion
        lastScanStartedAt = Date()
        if clearError { errorMessage = nil }
        scanningFolders.formUnion(targets.map(\.id))

        await withTaskGroup(of: (String, ScanResult?).self) { group in
            for folder in targets {
                group.addTask { [core] in
                    let result = try? await core.scanFolder(
                        path: folder.path,
                        onlyLocalChecks: folder.onlyLocalChecks
                    )
                    return (folder.id, result)
                }
            }
            for await (folderId, result) in group {
                // Supersession: a newer full scan owns the UI now, including
                // this folder's spinner — which is why the guard comes before
                // the removal below and not after it (§5.2).
                guard scanVersion == version else { continue }
                if let result { results[folderId] = result }
                scanningFolders.remove(folderId)
            }
        }

        guard scanVersion == version else { return }
        scanningFolders.subtract(targets.map(\.id))
        if isFullScan { isFullScanning = false }
    }

    /// Full scan of all folders, with global + per-folder progress indicators.
    /// Drives Scan All, the startup auto-scan and the window-focus rescan.
    func scanAll() async {
        await scan(folders: folders, isFullScan: true, clearError: true)
    }

    /// Scan a single folder (per-folder Scan control). On-demand, so it
    /// clears the shared error surface (§5.6).
    func scan(folder: MonitoredFolder) async {
        await scan(folders: [folder], isFullScan: false, clearError: true)
    }

    /// The monitored folders that actually contain the given repos. Pulling or
    /// cleaning a repo cannot change any other folder's state, so rescanning
    /// all of them was one full network scan per action. Falls back to every
    /// folder when a repo can't be attributed, so a miss is never a missed
    /// refresh.
    private func foldersForRepos(_ repoPaths: [String]) -> [MonitoredFolder] {
        var affected: [String: MonitoredFolder] = [:]
        for repoPath in repoPaths {
            // Longest match wins, so nested monitored folders attribute right.
            let best = folders
                .filter { Self.isInside(repoPath, $0.path) }
                .max { $0.path.count < $1.path.count }
            if let best { affected[best.id] = best }
        }
        return affected.isEmpty ? folders : Array(affected.values)
    }

    private nonisolated static func isInside(_ repoPath: String, _ folderPath: String) -> Bool {
        var base = folderPath
        while base.hasSuffix("/") { base.removeLast() }
        return repoPath == base || repoPath.hasPrefix(base + "/")
    }

    /// Window regained focus: rescan all folders with the normal scan
    /// indicators, throttled to once per 20s since the last scan of any kind
    /// (§5.1). Skipped while a scan is already in flight so it never supersedes
    /// one whose progress the user is watching.
    func appDidBecomeActive() {
        kanban.appDidBecomeActive()
        guard hasInitialScan, !folders.isEmpty else { return }
        guard !isFullScanning, scanningFolders.isEmpty else { return }
        if let last = lastScanStartedAt, Date().timeIntervalSince(last) < 20 { return }
        Task { await scanAll() }
    }

    // MARK: - Folder CRUD (FRONTEND.md §4)

    /// Returns a user-facing error message, or nil on success.
    func saveFolder(
        target: FolderFormTarget,
        path: String,
        name: String,
        onlyLocalChecks: Bool
    ) async -> String? {
        do {
            switch target {
            case .add:
                _ = try core.addMonitoredFolder(path: path, name: name, onlyLocalChecks: onlyLocalChecks)
            case .edit(let folder):
                try core.updateMonitoredFolder(
                    id: folder.id,
                    path: path,
                    name: name,
                    onlyLocalChecks: onlyLocalChecks
                )
            }
        } catch {
            NSLog("folder save failed: \(Self.message(error))")
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
                errorMessage = "Failed to delete folder"
                NSLog("folder delete failed: \(Self.message(error))")
                return
            }
            results.removeValue(forKey: folder.id)
            if selection == .folder(folder.id) { selection = .all }
            await loadFolders()
        }
    }

    // MARK: - Repo operations (FRONTEND.md §5.5)

    func pull(repoPath: String) async {
        pullingRepos.insert(repoPath)
        do {
            _ = try await core.pullRepo(path: repoPath)
            pullingRepos.remove(repoPath)
            await rescan(after: [repoPath])
        } catch {
            pullingRepos.remove(repoPath)
            errorMessage = "Failed to pull \(repoPath): \(Self.message(error))"
        }
    }

    func clean(repoPath: String) async {
        cleaningRepos.insert(repoPath)
        do {
            let result = try await core.cleanRepo(path: repoPath)
            cleaningRepos.remove(repoPath)
            // Set before the rescan, not after: the message used to appear only
            // once the rescan finished, then outlive it until the next scan.
            if result.filesRemoved.isEmpty && result.directoriesRemoved.isEmpty {
                errorMessage = "No ignored files to clean in \(Self.repoName(repoPath))"
            }
            await rescan(after: [repoPath])
        } catch {
            cleaningRepos.remove(repoPath)
            errorMessage = "Failed to clean \(repoPath): \(Self.message(error))"
        }
    }

    /// The rescan that follows a repo action: only the folders those repos live
    /// in, and never clearing the message the action just set.
    private func rescan(after repoPaths: [String]) async {
        await scan(folders: foldersForRepos(repoPaths), isFullScan: false, clearError: false)
    }

    func pullAll(_ repos: [RepoStatus]) async {
        guard !isBulkPulling else { return }
        isBulkPulling = true
        let paths = repos.map(\.path)
        pullingRepos.formUnion(paths)

        let failures = await runOnEach(paths) { [core] path in
            _ = try await core.pullRepo(path: path)
        }

        pullingRepos.subtract(paths)
        isBulkPulling = false
        if let first = failures.first {
            errorMessage = "Failed to pull \(failures.count) repo(s): \(first)"
        }
        await rescan(after: paths)
    }

    func cleanAll(_ repos: [RepoStatus]) async {
        guard !isBulkCleaning else { return }
        isBulkCleaning = true
        let paths = repos.map(\.path)
        cleaningRepos.formUnion(paths)

        let failures = await runOnEach(paths) { [core] path in
            _ = try await core.cleanRepo(path: path)
        }

        cleaningRepos.subtract(paths)
        isBulkCleaning = false
        if let first = failures.first {
            errorMessage = "Failed to clean \(failures.count) repo(s): \(first)"
        }
        await rescan(after: paths)
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
            return "Failed to save setting"
        }
    }

    func setDefaultEditor(_ id: String?) -> String? {
        do {
            try core.setDefaultEditor(editorId: id)
            defaultEditorId = id
            return nil
        } catch {
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
