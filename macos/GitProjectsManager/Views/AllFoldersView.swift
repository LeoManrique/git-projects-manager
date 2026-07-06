import SwiftUI

/// Dashboard listing every monitored folder with its actionable repositories
/// expanded inline, so uncommitted / unpushed / unpulled work is visible at a
/// glance without drilling into each folder (FRONTEND.md §5.3). Clean repos are
/// summarized in the header and shown in full in the per-folder detail view.
struct AllFoldersView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        if model.folders.isEmpty {
            ContentUnavailableView {
                Label("No Folders Configured", systemImage: "folder.badge.plus")
            } description: {
                Text("Add a folder to start monitoring its git repositories.")
            } actions: {
                Button("Add Folder") { model.folderForm = .add }
                    .buttonStyle(.glassProminent)
            }
            .navigationTitle("All Folders")
        } else {
            List {
                ForEach(model.folders, id: \.id) { folder in
                    FolderOverviewSection(folder: folder)
                }
            }
            .scrollEdgeEffectStyle(.soft, for: .top)
            .navigationTitle("All Folders")
        }
    }
}

/// One monitored folder in the overview: a summary header plus its actionable
/// repos (every category except Clean) grouped and expanded inline.
struct FolderOverviewSection: View {
    @Environment(AppModel.self) private var model
    let folder: MonitoredFolder

    private var isScanning: Bool { model.scanningFolders.contains(folder.id) }

    /// Actionable categories (Clean excluded) that still have repos after the
    /// active search filter, in display order.
    private var actionableGroups: [(RepoCategory, [RepoStatus])] {
        guard let result = model.results[folder.id] else { return [] }
        return RepoCategory.allCases
            .filter { $0 != .clean }
            .compactMap { category in
                let repos = model.filtered(category.repos(in: result))
                return repos.isEmpty ? nil : (category, repos)
            }
    }

    var body: some View {
        Section {
            content
        } header: {
            FolderSummaryHeader(folder: folder)
        }
    }

    @ViewBuilder
    private var content: some View {
        if let result = model.results[folder.id] {
            let groups = actionableGroups
            if groups.isEmpty {
                emptyRow(result)
            } else {
                ForEach(groups, id: \.0) { category, repos in
                    SectionHeader(category: category, repos: repos)
                        .padding(.top, 6)
                        .listRowSeparator(.hidden)
                    ForEach(repos, id: \.path) { repo in
                        RepoRowView(repo: repo, category: category)
                    }
                }
            }
        } else if isScanning {
            HStack(spacing: 8) {
                ProgressView().controlSize(.small)
                Text("Scanning…").foregroundStyle(.secondary)
            }
        } else {
            Button("Scan Folder") { Task { await model.scan(folder: folder) } }
        }
    }

    @ViewBuilder
    private func emptyRow(_ result: ScanResult) -> some View {
        if !model.searchText.isEmpty {
            Text("No matching repositories")
                .font(.callout)
                .foregroundStyle(.secondary)
        } else {
            Label(
                "All \(result.totalRepositories) repositories are clean",
                systemImage: "checkmark.circle.fill"
            )
            .font(.callout)
            .foregroundStyle(.green)
        }
    }
}

/// Sticky per-folder header for the overview: name, path, summary, and the
/// per-folder scan / open-detail controls.
struct FolderSummaryHeader: View {
    @Environment(AppModel.self) private var model
    let folder: MonitoredFolder

    private var isScanning: Bool { model.scanningFolders.contains(folder.id) }

    var body: some View {
        HStack(spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text(folder.name)
                    .font(.headline)
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                Text(folder.path)
                    .font(.caption)
                    .monospaced()
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer(minLength: 12)

            statusArea

            Button {
                Task { await model.scan(folder: folder) }
            } label: {
                Image(systemName: "arrow.clockwise")
            }
            .buttonStyle(.borderless)
            .disabled(isScanning)
            .help("Scan this folder")

            Button {
                model.selection = .folder(folder.id)
            } label: {
                Image(systemName: "arrow.up.forward.square")
            }
            .buttonStyle(.borderless)
            .help("Open \(folder.name)")
        }
        .padding(.vertical, 4)
        .contextMenu {
            Button("Scan") { Task { await model.scan(folder: folder) } }
                .disabled(isScanning)
            Button("Open Folder") { model.selection = .folder(folder.id) }
            Button("Edit…") { model.folderForm = .edit(folder) }
            Button("Show in Finder") { model.revealInFinder(folder.path) }
            Divider()
            Button("Remove", role: .destructive) { model.deleteFolder(folder) }
        }
    }

    @ViewBuilder
    private var statusArea: some View {
        if isScanning {
            HStack(spacing: 6) {
                ProgressView().controlSize(.small)
                Text("Scanning…")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
        } else if let result = model.results[folder.id] {
            HStack(spacing: 8) {
                Text("\(result.totalRepositories) repos")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                CategoryBadge(count: result.clean.count, category: .clean)
            }
        } else {
            Text("Not scanned")
                .font(.callout)
                .foregroundStyle(.tertiary)
        }
    }
}

/// Count pill in a category color (FRONTEND.md §5.3). Counts always reflect
/// the unfiltered scan result.
struct CategoryBadge: View {
    let count: Int
    let category: RepoCategory

    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(category.color)
                .frame(width: 6, height: 6)
            Text("\(count) \(category.badgeLabel)")
        }
        .font(.caption)
        .foregroundStyle(count == 0 ? AnyShapeStyle(.secondary) : AnyShapeStyle(category.color))
        .padding(.horizontal, 7)
        .padding(.vertical, 2)
        .background(category.color.opacity(count == 0 ? 0.06 : 0.13), in: Capsule())
    }
}
