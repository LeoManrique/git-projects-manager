import SwiftUI

struct SidebarView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        @Bindable var model = model
        List(selection: $model.selection) {
            Label("All Folders", systemImage: "square.grid.2x2")
                .tag(SidebarItem.all)

            Label("Kanban", systemImage: "rectangle.split.3x1")
                .tag(SidebarItem.kanban)

            Section("Folders") {
                ForEach(model.folders, id: \.id) { folder in
                    sidebarRow(folder)
                }
            }
        }
        .safeAreaInset(edge: .bottom, spacing: 0) {
            addFolderButton
        }
        .navigationTitle("Git Projects")
    }

    private func sidebarRow(_ folder: MonitoredFolder) -> some View {
        HStack {
            Label(folder.name, systemImage: "folder")
                .lineLimit(1)
            Spacer()
            if model.scanningFolders.contains(folder.id) {
                ProgressView()
                    .controlSize(.mini)
            } else {
                let attention = model.attentionCount(for: folder.id)
                if attention > 0 {
                    Text("\(attention)")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 1)
                        .background(.quaternary, in: Capsule())
                }
            }
        }
        .tag(SidebarItem.folder(folder.id))
        .contextMenu {
            Button("Scan") {
                Task { await model.scan(folder: folder) }
            }
            .disabled(model.scanningFolders.contains(folder.id))
            Button("Edit…") { model.folderForm = .edit(folder) }
            Button("Show in Finder") { model.revealInFinder(folder.path) }
            Divider()
            Button("Remove", role: .destructive) { model.deleteFolder(folder) }
        }
    }

    private var addFolderButton: some View {
        HStack {
            Button {
                model.folderForm = .add
            } label: {
                Label("Add Folder", systemImage: "plus.circle")
            }
            .buttonStyle(.borderless)
            Spacer()
        }
        .padding(10)
    }
}
