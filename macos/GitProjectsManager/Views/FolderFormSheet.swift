import SwiftUI

/// Add/Edit form for a monitored folder (FRONTEND.md §4).
struct FolderFormSheet: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    let target: FolderFormTarget

    @State private var path = ""
    @State private var name = ""
    @State private var onlyLocalChecks = false
    @State private var validationError: String?
    @State private var isSaving = false
    @State private var showPicker = false

    private var isEdit: Bool {
        if case .edit = target { return true }
        return false
    }

    var body: some View {
        VStack(spacing: 0) {
            Form {
                Section {
                    LabeledContent("Folder Path") {
                        HStack(spacing: 6) {
                            TextField(
                                "",
                                text: $path,
                                prompt: Text("e.g. /Users/you/Projects")
                            )
                            .monospaced()
                            .labelsHidden()
                            Button("Choose…") { showPicker = true }
                        }
                    }
                    TextField(
                        "Display Name",
                        text: $name,
                        prompt: Text("e.g. My Projects")
                    )
                }

                Section {
                    Toggle("Only local checks", isOn: $onlyLocalChecks)
                    Text("Skips remote fetch and push/pull checks for faster, offline-friendly scans.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .formStyle(.grouped)

            Divider()

            HStack {
                if let validationError {
                    Label(validationError, systemImage: "exclamationmark.circle")
                        .font(.callout)
                        .foregroundStyle(.red)
                }
                Spacer()
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .disabled(isSaving)
                Button(submitLabel) { submit() }
                    .buttonStyle(.glassProminent)
                    .keyboardShortcut(.defaultAction)
                    .disabled(isSaving)
            }
            .padding(12)
        }
        .frame(width: 520)
        .fileImporter(
            isPresented: $showPicker,
            allowedContentTypes: [.folder]
        ) { result in
            if case .success(let url) = result {
                path = url.path
            }
        }
        .onAppear(perform: populate)
    }

    private var submitLabel: String {
        if isSaving { return isEdit ? "Save…" : "Add…" }
        return isEdit ? "Save" : "Add"
    }

    private func populate() {
        if case .edit(let folder) = target {
            path = folder.path
            name = folder.name
            onlyLocalChecks = folder.onlyLocalChecks
        }
    }

    private func submit() {
        let trimmedPath = path.trimmingCharacters(in: .whitespaces)
        let trimmedName = name.trimmingCharacters(in: .whitespaces)
        guard !trimmedPath.isEmpty, !trimmedName.isEmpty else {
            validationError = "Path and name are required"
            return
        }
        validationError = nil
        isSaving = true
        Task {
            let error = await model.saveFolder(
                target: target,
                path: path,
                name: name,
                onlyLocalChecks: onlyLocalChecks
            )
            isSaving = false
            if let error {
                validationError = error
            } else {
                dismiss()
            }
        }
    }
}
