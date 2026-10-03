import SwiftUI

/// The repo categories, in display order — the transient Checking first,
/// Clean last (FRONTEND.md §5.3). Unpublished, Remote Not Found and Unknown
/// Remote State are overlays: a repo in any of them also appears in its
/// primary category.
enum RepoCategory: String, CaseIterable, Identifiable {
    case checking
    case changes
    case unpushed
    case unpulled
    case unpublished
    case remoteNotFound
    case remoteStateUnknown
    case uninitialized
    case errors
    case clean

    var id: String { rawValue }

    var title: String {
        switch self {
        case .checking: "Checking"
        case .changes: "Uncommitted Changes"
        case .unpushed: "Unpushed Commits"
        case .unpulled: "Unpulled Commits"
        case .unpublished: "Unpublished"
        case .remoteNotFound: "Remote Not Found"
        case .remoteStateUnknown: "Unknown Remote State"
        case .clean: "Clean"
        case .uninitialized: "Uninitialized"
        case .errors: "Errors"
        }
    }

    /// Short label used in folder badge summaries.
    var badgeLabel: String {
        switch self {
        case .checking: "checking"
        case .changes: "changed"
        case .unpushed: "unpushed"
        case .unpulled: "unpulled"
        case .unpublished: "unpublished"
        case .remoteNotFound: "remote gone"
        case .remoteStateUnknown: "unknown"
        case .clean: "clean"
        case .uninitialized: "uninitialized"
        case .errors: "errors"
        }
    }

    var color: Color {
        switch self {
        case .checking: .gray
        case .changes: .yellow
        case .unpushed: .orange
        case .unpulled: .purple
        case .unpublished: .blue
        case .remoteNotFound: .pink
        case .remoteStateUnknown: .gray
        case .clean: .green
        case .uninitialized: .gray
        case .errors: .red
        }
    }

    /// Checking, Clean and Uninitialized render dimmed (FRONTEND.md §5.3).
    var isMuted: Bool { self == .checking || self == .clean || self == .uninitialized }

    /// Checking holds the repos a scan found but has no status for yet, so a
    /// silent scan keeps them out of sight until their status lands.
    var onlyWhileScanning: Bool { self == .checking }

    /// Fetch & Pull is hidden for Checking (no status yet), Uninitialized,
    /// Unpublished (no remote), and Remote Not Found (remote is gone),
    /// visible-but-disabled for Changes/Errors, enabled elsewhere
    /// (FRONTEND.md §5.5).
    var showsPull: Bool { ![.checking, .uninitialized, .unpublished, .remoteNotFound].contains(self) }
    /// Pull stays enabled for Unknown Remote State: the comparison failed, but
    /// pulling is how a user resolves it and `git pull` reports its own errors.
    var pullEnabled: Bool { self == .unpushed || self == .unpulled || self == .clean || self == .remoteStateUnknown }

    /// Clean Ignored Files is offered only in the Clean section.
    var showsClean: Bool { self == .clean }

    /// Sections with a bulk action (FRONTEND.md §5.3).
    var hasBulkPull: Bool { self == .unpulled }
    var hasBulkClean: Bool { self == .clean }

    /// Badge order in folder summaries (FRONTEND.md §5.3).
    static let badgeOrder: [RepoCategory] = [.clean, .changes, .unpushed, .unpulled, .unpublished, .remoteNotFound, .remoteStateUnknown, .uninitialized]

    func repos(in result: ScanResult) -> [RepoStatus] {
        switch self {
        case .checking: result.checking
        case .changes: result.withChanges
        case .unpushed: result.withUnpushed
        case .unpulled: result.withUnpulled
        case .unpublished: result.unpublished
        case .remoteNotFound: result.remoteNotFound
        case .remoteStateUnknown: result.remoteStateUnknown
        case .clean: result.clean
        case .uninitialized: result.uninitialized
        case .errors: result.errors
        }
    }
}
