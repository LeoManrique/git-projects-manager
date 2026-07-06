import SwiftUI

/// Scan results for a single monitored folder: the six fixed category
/// sections (FRONTEND.md §5.3).
struct FolderDetailView: View {
    @Environment(AppModel.self) private var model
    let folder: MonitoredFolder

    private var isScanning: Bool { model.scanningFolders.contains(folder.id) }

    var body: some View {
        Group {
            if let result = model.results[folder.id] {
                RepoSectionsView(result: result)
            } else if isScanning {
                VStack(spacing: 12) {
                    ProgressView()
                    Text("Scanning…").foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ContentUnavailableView {
                    Label("Not Scanned", systemImage: "magnifyingglass")
                } description: {
                    Text("Scan this folder to see the status of its repositories.")
                } actions: {
                    Button("Scan Folder") {
                        Task { await model.scan(folder: folder) }
                    }
                    .buttonStyle(.glassProminent)
                }
            }
        }
        .navigationTitle(folder.name)
        .navigationSubtitle(folder.path)
        .toolbar {
            ToolbarItem {
                Button {
                    Task { await model.scan(folder: folder) }
                } label: {
                    Label("Scan Folder", systemImage: "arrow.clockwise.circle")
                }
                .disabled(isScanning)
                .help("Rescan \(folder.name)")
            }
        }
    }
}

struct RepoSectionsView: View {
    @Environment(AppModel.self) private var model
    let result: ScanResult

    private var visibleSections: [(RepoCategory, [RepoStatus])] {
        RepoCategory.allCases.compactMap { category in
            let repos = model.filtered(category.repos(in: result))
            return repos.isEmpty ? nil : (category, repos)
        }
    }

    var body: some View {
        let sections = visibleSections
        if sections.isEmpty, !model.searchText.isEmpty {
            ContentUnavailableView.search(text: model.searchText)
        } else {
            List {
                ForEach(sections, id: \.0) { category, repos in
                    Section {
                        ForEach(repos, id: \.path) { repo in
                            RepoRowView(repo: repo, category: category)
                        }
                    } header: {
                        SectionHeader(category: category, repos: repos)
                    }
                }

                Section {
                    Text("Completed in \(result.executionTime, specifier: "%.2f")s")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                        .listRowSeparator(.hidden)
                }
            }
            .scrollEdgeEffectStyle(.soft, for: .top)
        }
    }
}

struct SectionHeader: View {
    @Environment(AppModel.self) private var model
    let category: RepoCategory
    let repos: [RepoStatus]

    var body: some View {
        HStack {
            Label("\(category.title) (\(repos.count))", systemImage: category.symbol)
                .font(.callout.weight(.semibold))
                .foregroundStyle(category.color)

            Spacer()

            // Bulk actions act on the filtered list (FRONTEND.md §5.4).
            if category.hasBulkPull {
                bulkMenu(
                    label: "Fetch & Pull All (\(repos.count))",
                    inFlight: model.isBulkPulling
                ) {
                    Task { await model.pullAll(repos) }
                }
            } else if category.hasBulkClean {
                bulkMenu(
                    label: "Clean All (\(repos.count))",
                    inFlight: model.isBulkCleaning
                ) {
                    Task { await model.cleanAll(repos) }
                }
            }
        }
    }

    @ViewBuilder
    private func bulkMenu(label: String, inFlight: Bool, action: @escaping () -> Void) -> some View {
        if inFlight {
            ProgressView().controlSize(.small)
        } else {
            Menu {
                Button(label, action: action)
            } label: {
                Image(systemName: "ellipsis.circle")
            }
            .menuIndicator(.hidden)
            .buttonStyle(.borderless)
            .fixedSize()
        }
    }
}
