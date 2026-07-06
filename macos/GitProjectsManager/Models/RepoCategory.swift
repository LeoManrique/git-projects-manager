import SwiftUI

/// The six fixed repo categories, in display order (FRONTEND.md §5.3).
enum RepoCategory: String, CaseIterable, Identifiable {
    case changes
    case unpushed
    case unpulled
    case clean
    case uninitialized
    case errors

    var id: String { rawValue }

    var title: String {
        switch self {
        case .changes: "Uncommitted Changes"
        case .unpushed: "Unpushed Commits"
        case .unpulled: "Unpulled Commits"
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
        case .clean: .green
        case .uninitialized: .gray
        case .errors: .red
        }
    }

    var symbol: String {
        switch self {
        case .changes: "pencil.circle.fill"
        case .unpushed: "arrow.up.circle.fill"
        case .unpulled: "arrow.down.circle.fill"
        case .clean: "checkmark.circle.fill"
        case .uninitialized: "questionmark.circle.fill"
        case .errors: "exclamationmark.triangle.fill"
        }
    }

    /// Clean and Uninitialized render dimmed (FRONTEND.md §5.3).
    var isMuted: Bool { self == .clean || self == .uninitialized }

    /// Fetch & Pull is hidden for Uninitialized, visible-but-disabled for
    /// Changes/Errors, enabled elsewhere (FRONTEND.md §5.5).
    var showsPull: Bool { self != .uninitialized }
    var pullEnabled: Bool { self == .unpushed || self == .unpulled || self == .clean }

    /// Clean Ignored Files is offered only in the Clean section.
    var showsClean: Bool { self == .clean }

    /// Sections with a bulk action (FRONTEND.md §5.3).
    var hasBulkPull: Bool { self == .unpulled }
    var hasBulkClean: Bool { self == .clean }

    /// Badge order in folder summaries (FRONTEND.md §5.3).
    static let badgeOrder: [RepoCategory] = [.clean, .changes, .unpushed, .unpulled, .uninitialized]

    func repos(in result: ScanResult) -> [RepoStatus] {
        switch self {
        case .changes: result.withChanges
        case .unpushed: result.withUnpushed
        case .unpulled: result.withUnpulled
        case .clean: result.clean
        case .uninitialized: result.uninitialized
        case .errors: result.errors
        }
    }
}
