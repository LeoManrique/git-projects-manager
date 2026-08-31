import SwiftUI

struct MainWindow: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        @Bindable var model = model
        NavigationSplitView {
            SidebarView()
                .navigationSplitViewColumnWidth(min: 210, ideal: 250)
        } detail: {
            DetailView()
        }
        .searchable(text: $model.searchText, prompt: "Search repositories")
        .sheet(item: $model.folderForm) { target in
            FolderFormSheet(target: target)
        }
        .frame(minWidth: 800, minHeight: 540)
    }
}

struct DetailView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        Group {
            switch model.selection {
            case .kanban:
                KanbanBoardView()
            case .folder(let id):
                if let folder = model.folder(withId: id) {
                    FolderDetailView(folder: folder)
                } else {
                    AllFoldersView()
                }
            default:
                AllFoldersView()
            }
        }
        .safeAreaInset(edge: .top, spacing: 0) {
            ErrorBanner()
        }
        .toolbar {
            if model.selection == .kanban {
                ToolbarItem(placement: .secondaryAction) {
                    SyncStatusChip()
                }
                ToolbarItem(placement: .primaryAction) {
                    refreshBoardButton
                }
            } else {
                if let notice = localChecksNotice {
                    ToolbarItem(placement: .secondaryAction) {
                        LocalChecksChip(notice: notice)
                    }
                }
                ToolbarItem(placement: .primaryAction) {
                    ScanButton()
                }
            }
        }
    }

    /// Which of the folders in view skip the network, so "no unpushed commits"
    /// is never read as a finding when it was never checked (FRONTEND.md §5.4).
    private var localChecksNotice: String? {
        let inView = model.foldersInView
        let localOnly = inView.filter(\.onlyLocalChecks).count
        guard localOnly > 0 else { return nil }
        guard localOnly < inView.count else { return "Local checks only" }
        return "\(localOnly) of \(inView.count) folders: local checks only"
    }

    private var refreshBoardButton: some View {
        Button {
            Task { await model.kanban.refresh() }
        } label: {
            if model.kanban.isRefreshing {
                Label {
                    Text("Refreshing…")
                } icon: {
                    ProgressView().controlSize(.small)
                }
            } else {
                Label("Refresh", systemImage: "arrow.clockwise")
            }
        }
        .buttonStyle(.glassProminent)
        .disabled(model.kanban.isRefreshing)
        .help("Refresh repositories and sync the board")
    }
}

/// Toolbar chip showing the kanban sync status; its menu hosts the
/// sign-in/out shortcuts (full account management lives in Settings).
/// Toolbar indicator for folders configured with "Only local checks", so that
/// "no unpushed commits" is never read as a finding when the scan never asked
/// (FRONTEND.md §5.4).
struct LocalChecksChip: View {
    let notice: String

    var body: some View {
        Label(notice, systemImage: "wifi.slash")
            .foregroundStyle(.secondary)
            .help("Fetch, unpushed and unpulled checks are skipped for these folders. Turn off \"Only local checks\" in the folder's settings to include them.")
    }
}

struct SyncStatusChip: View {
    @Environment(AppModel.self) private var model

    @State private var isBusy = false
    @State private var signInError: String?

    var body: some View {
        let kanban = model.kanban
        Menu {
            if let user = kanban.syncUser {
                Text(user.name ?? user.email ?? user.sub)
                if let email = user.email, user.name != nil {
                    Text(email)
                }
                Divider()
                Button("Sign Out") {
                    Task { await kanban.signOut() }
                }
            } else {
                Button(isBusy ? "Waiting for browser…" : "Sign in with Google…") {
                    Task {
                        isBusy = true
                        signInError = await kanban.signIn()
                        isBusy = false
                    }
                }
                .disabled(isBusy)
                if let signInError {
                    Text(signInError)
                }
            }
        } label: {
            Label(kanban.syncStatus.displayLabel, systemImage: kanban.syncStatus.symbol)
        }
        .help("Kanban cloud sync: \(kanban.syncStatus.displayLabel)")
    }
}

/// Shared, non-dismissible error surface; cleared when the next on-demand
/// scan starts (FRONTEND.md §5.6).
struct ErrorBanner: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        if let message = model.errorMessage {
            HStack(spacing: 8) {
                Image(systemName: "exclamationmark.triangle.fill")
                Text(message)
                    .lineLimit(2)
                    .truncationMode(.middle)
                Spacer(minLength: 0)
            }
            .font(.callout)
            .foregroundStyle(.red)
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(.red.opacity(0.1), in: RoundedRectangle(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .strokeBorder(.red.opacity(0.25), lineWidth: 1)
            )
            .padding(.horizontal, 10)
            .padding(.top, 8)
        }
    }
}
