import SwiftUI

/// The eight fixed repo categories, in display order — Clean last
/// (FRONTEND.md §5.3). Unpublished and Remote Not Found are overlays: a repo in
/// either also appears in its primary status category.
enum RepoCategory: String, CaseIterable, Identifiable {
    case changes
    case unpushed
    case unpulled
    case unpublished
    case remoteNotFound
    case uninitialized
    case errors
    case clean

    var id: String { rawValue }

    var title: String {
        switch self {
        case .changes: "Uncommitted Changes"
        case .unpushed: "Unpushed Commits"
        case .unpulled: "Unpulled Commits"
        case .unpublished: "Unpublished"
        case .remoteNotFound: "Remote Not Found"
        case .clean: "Clean"
        case .uninitialized: "Uninitialized"
        case .errors: "Errors"
        }
    }

    /// Short label used in folder badge summaries.
    var badgeLabel: String {
        switch self {
        case .changes: "changed"
        case .unpushed: "unpushed"
        case .unpulled: "unpulled"
        case .unpublished: "unpublished"
        case .remoteNotFound: "remote gone"
        case .clean: "clean"
        case .uninitialized: "uninitialized"
        case .errors: "errors"
        }
    }

    var color: Color {
        switch self {
        case .changes: .yellow
        case .unpushed: .orange
        case .unpulled: .purple
        case .unpublished: .blue
        case .remoteNotFound: .pink
        case .clean: .green
        case .uninitialized: .gray
        case .errors: .red
        }
    }

    /// Clean and Uninitialized render dimmed (FRONTEND.md §5.3).
    var isMuted: Bool { self == .clean || self == .uninitialized }

    /// Fetch & Pull is hidden for Uninitialized, Unpublished (no remote), and
    /// Remote Not Found (remote is gone), visible-but-disabled for
    /// Changes/Errors, enabled elsewhere (FRONTEND.md §5.5).
    var showsPull: Bool { self != .uninitialized && self != .unpublished && self != .remoteNotFound }
    var pullEnabled: Bool { self == .unpushed || self == .unpulled || self == .clean }

    /// Clean Ignored Files is offered only in the Clean section.
    var showsClean: Bool { self == .clean }

    /// Sections with a bulk action (FRONTEND.md §5.3).
    var hasBulkPull: Bool { self == .unpulled }
    var hasBulkClean: Bool { self == .clean }

    /// Badge order in folder summaries (FRONTEND.md §5.3).
    static let badgeOrder: [RepoCategory] = [.clean, .changes, .unpushed, .unpulled, .unpublished, .remoteNotFound, .uninitialized]

    func repos(in result: ScanResult) -> [RepoStatus] {
        switch self {
        case .changes: result.withChanges
        case .unpushed: result.withUnpushed
        case .unpulled: result.withUnpulled
        case .unpublished: result.unpublished
        case .remoteNotFound: result.remoteNotFound
        case .clean: result.clean
        case .uninitialized: result.uninitialized
        case .errors: result.errors
        }
    }
}
