import Foundation
import SwiftUI

/// The five fixed kanban columns, in display order. Raw values are the
/// on-disk/wire column ids shared with the Tauri app and sync server.
enum KanbanColumn: String, CaseIterable, Identifiable {
    case backlog
    case activeLow = "active-low"
    case activeHigh = "active-high"
    case done
    case closed

    var id: String { rawValue }

    var title: String {
        switch self {
        case .backlog: "Backlog"
        case .activeLow: "Active · Low"
        case .activeHigh: "Active · High"
        case .done: "Done"
        case .closed: "Closed"
        }
    }

    var color: Color {
        switch self {
        case .backlog: .gray
        case .activeLow: .blue
        case .activeHigh: .red
        case .done: .green
        case .closed: .yellow
        }
    }
}

/// A card joined with its GitHub repo metadata — the unit the board renders.
struct KanbanEntry: Identifiable {
    let card: KanbanCard
    let repo: GhRepo

    var id: String { card.nameWithOwner }

    var pushedDate: Date? {
        guard let pushedAt = repo.pushedAt else { return nil }
        return try? Date(pushedAt, strategy: .iso8601)
    }
}

/// Kanban board state and orchestration over the gpm-core FFI, mirroring the
/// Tauri app's behavior: cache-first paint, GitHub-repo-backed cards, and
/// optional cloud sync.
@MainActor
@Observable
final class KanbanModel {
    private let core: GpmCore

    // Data
    private(set) var auth: GhAuthStatus?
    private(set) var repos: [GhRepo] = []
    private(set) var state: KanbanState?
    private(set) var syncStatus: SyncStatus = .disabled
    private(set) var syncUser: SyncUser?

    // UI state
    private(set) var isLoading = true
    private(set) var isRefreshing = false
    var errorMessage: String?

    private var hasStarted = false
    private var lastRefreshAt: Date?
    /// Focus-triggered refreshes are debounced (matches the Tauri app).
    private static let focusRefreshDebounce: TimeInterval = 1.5

    init(core: GpmCore) {
        self.core = core
        syncUser = core.getSyncUser()
    }

    var authedGhUser: String? {
        if case .ok(let user)? = auth { return user }
        return nil
    }

    /// Cache-first paint, then revalidate in the background. Called on the
    /// board's first appearance; the full-screen spinner only shows on a
    /// genuine first launch with no cache.
    func startIfNeeded() {
        guard !hasStarted else { return }
        hasStarted = true
        if let local = try? core.loadKanbanLocal() {
            repos = local.repos
            state = local.state
            syncStatus = local.syncStatus
            isLoading = false
        }
        Task { await refresh() }
    }

    /// Full refresh: gh auth check, repo listing, board reconciliation, and
    /// cloud sync when signed in.
    func refresh() async {
        guard !isRefreshing else { return }
        isRefreshing = true
        errorMessage = nil
        defer {
            isLoading = false
            isRefreshing = false
            lastRefreshAt = Date()
        }

        let status: GhAuthStatus
        do {
            status = try await core.checkGhAuth()
        } catch {
            status = .error(message: AppModel.message(error))
        }
        auth = status
        guard case .ok = status else {
            repos = []
            return
        }

        do {
            let result = try await core.refreshKanban()
            repos = result.repos
            state = result.state
            syncStatus = result.syncStatus
            // A refresh can invalidate an expired session in the background.
            syncUser = core.getSyncUser()
        } catch {
            errorMessage = AppModel.message(error)
        }
    }

    /// Window regained focus: silent revalidation, debounced.
    func appDidBecomeActive() {
        guard hasStarted else { return }
        if let last = lastRefreshAt,
           Date().timeIntervalSince(last) < Self.focusRefreshDebounce { return }
        Task { await refresh() }
    }

    /// Optimistic column move; the store write and background one-card sync
    /// happen in core. Failure refetches authoritative state.
    func move(_ nameWithOwner: String, to column: KanbanColumn) {
        guard var updated = state,
              var card = updated.cards[nameWithOwner],
              card.column != column.rawValue
        else { return }

        card.column = column.rawValue
        card.updatedAt = Int64(Date().timeIntervalSince1970 * 1000)
        updated.cards[nameWithOwner] = card
        state = updated

        Task {
            do {
                state = try await core.moveKanbanCard(
                    nameWithOwner: nameWithOwner,
                    toColumn: column.rawValue
                )
            } catch {
                errorMessage = AppModel.message(error)
                await refresh()
            }
        }
    }

    /// Permanently delete the repository on GitHub and rebuild the board.
    func deleteRepo(_ nameWithOwner: String) async {
        errorMessage = nil
        do {
            let result = try await core.deleteGithubRepo(nameWithOwner: nameWithOwner)
            repos = result.repos
            state = result.state
            syncStatus = result.syncStatus
        } catch {
            errorMessage = AppModel.message(error)
        }
    }

    // MARK: - Account

    /// Returns a user-facing error message, or nil on success.
    func signIn() async -> String? {
        do {
            syncUser = try await core.signInWithGoogle()
            await refresh()
            return nil
        } catch {
            return AppModel.message(error)
        }
    }

    func signOut() async {
        await core.signOut()
        syncUser = nil
        await refresh()
    }

    // MARK: - Derived

    /// Cards joined with repos, grouped by column, sorted by name, filtered
    /// by the search query (case-insensitive substring on owner/repo).
    func board(matching query: String) -> [KanbanColumn: [KanbanEntry]] {
        var result: [KanbanColumn: [KanbanEntry]] = [:]
        for column in KanbanColumn.allCases { result[column] = [] }
        guard let state else { return result }

        let repoByName = Dictionary(
            repos.map { ($0.nameWithOwner, $0) },
            uniquingKeysWith: { first, _ in first }
        )
        let trimmed = query.trimmingCharacters(in: .whitespaces).lowercased()

        for card in state.cards.values {
            guard let repo = repoByName[card.nameWithOwner] else { continue }
            if !trimmed.isEmpty, !repo.nameWithOwner.lowercased().contains(trimmed) { continue }
            // Unknown/legacy column ids fall back to Backlog.
            let column = KanbanColumn(rawValue: card.column) ?? .backlog
            result[column, default: []].append(KanbanEntry(card: card, repo: repo))
        }
        for column in KanbanColumn.allCases {
            result[column]?.sort {
                $0.repo.nameWithOwner.localizedCaseInsensitiveCompare($1.repo.nameWithOwner)
                    == .orderedAscending
            }
        }
        return result
    }

    // MARK: - Open actions

    func openOnGitHub(_ repo: GhRepo) {
        open(url: repo.url)
    }

    func open(url: String) {
        do {
            try core.openUrl(url: url)
        } catch {
            errorMessage = "Failed to open browser: \(AppModel.message(error))"
        }
    }
}

extension SyncStatus {
    var displayLabel: String {
        switch self {
        case .disabled: "Sync Off"
        case .synced: "Synced"
        case .offline: "Offline"
        case .expired: "Session Expired"
        }
    }

    var symbol: String {
        switch self {
        case .disabled: "icloud.slash"
        case .synced: "checkmark.icloud"
        case .offline: "wifi.slash"
        case .expired: "exclamationmark.icloud"
        }
    }

    var tint: Color {
        switch self {
        case .disabled: .secondary
        case .synced: .green
        case .offline: .yellow
        case .expired: .orange
        }
    }
}
