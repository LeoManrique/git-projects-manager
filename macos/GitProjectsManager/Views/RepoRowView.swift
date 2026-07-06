import SwiftUI

struct RepoRowView: View {
    @Environment(AppModel.self) private var model
    let repo: RepoStatus
    let category: RepoCategory

    @State private var isHovering = false

    private var isBusy: Bool { model.isBusy(repoPath: repo.path) }

    var body: some View {
        HStack(spacing: 10) {
            // Softer than the full-strength section-header dot.
            Circle()
                .fill(category.color.opacity(0.5))
                .frame(width: 8, height: 8)

            VStack(alignment: .leading, spacing: 2) {
                PathText(path: repo.path, branch: repo.branch, muted: category.isMuted)
                if category == .errors, let message = repo.errorMessage {
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .lineLimit(2)
                }
            }
            // Claim the slack (instead of the Spacer) so PathText measures
            // the true available width.
            .layoutPriority(1)

            Spacer(minLength: 8)

            if isBusy {
                ProgressView().controlSize(.small)
            } else if isHovering {
                Menu {
                    RepoActions(repo: repo, category: category)
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
                .menuIndicator(.hidden)
                .buttonStyle(.borderless)
                .fixedSize()
            }
        }
        .padding(.vertical, 2)
        .contentShape(Rectangle())
        .onHover { isHovering = $0 }
        .contextMenu {
            RepoActions(repo: repo, category: category)
        }
    }
}

/// Per-repo actions, shared between the hover menu and the context menu
/// (FRONTEND.md §5.5).
struct RepoActions: View {
    @Environment(AppModel.self) private var model
    let repo: RepoStatus
    let category: RepoCategory

    private var isBusy: Bool { model.isBusy(repoPath: repo.path) }

    var body: some View {
        if let editor = model.defaultEditor {
            Button("Open in \(editor.displayName)") { model.openInEditor(repo.path) }
        }
        if let terminal = model.defaultTerminal {
            Button("Open in \(terminal.displayName)") { model.openInTerminal(repo.path) }
        }
        Button("Open in LMS Github") { model.openInLmsGithub(repo.path) }
        Button("Show in Finder") { model.revealInFinder(repo.path) }

        if category.showsPull || category.showsClean {
            Divider()
        }
        if category.showsPull {
            Button("Fetch & Pull") {
                Task { await model.pull(repoPath: repo.path) }
            }
            .disabled(!category.pullEnabled || isBusy)
        }
        if category.showsClean {
            Button("Clean Ignored Files") {
                Task { await model.clean(repoPath: repo.path) }
            }
            .disabled(isBusy)
        }
    }
}
