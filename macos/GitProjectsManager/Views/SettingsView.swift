import SwiftUI

struct SettingsView: View {
    var body: some View {
        TabView {
            Tab("Default Apps", systemImage: "macwindow") {
                DefaultAppsSettingsView()
            }
            Tab("Git Clean", systemImage: "paintbrush") {
                GitCleanSettingsView()
            }
            Tab("Account", systemImage: "person.crop.circle") {
                AccountSettingsView()
            }
        }
        .frame(width: 520)
    }
}

/// Google sign-in for kanban cloud sync (FRONTEND.md §6.3). The app works
/// fully without signing in — sync is opt-in.
struct AccountSettingsView: View {
    @Environment(AppModel.self) private var model

    @State private var busy = false
    @State private var error: String?

    var body: some View {
        Form {
            Section("Cloud Sync") {
                Text("Sign in with Google to sync your kanban board across devices. The app works fully without signing in — sync is opt-in.")
                    .font(.callout)
                    .foregroundStyle(.secondary)

                if let user = model.kanban.syncUser {
                    LabeledContent("Signed in as", value: user.name ?? user.email ?? user.sub)
                    if let email = user.email, user.name != nil {
                        LabeledContent("Email", value: email)
                    }
                    Button(busy ? "Signing out…" : "Sign Out") {
                        Task {
                            busy = true
                            error = nil
                            await model.kanban.signOut()
                            busy = false
                        }
                    }
                    .disabled(busy)
                } else {
                    Button(busy ? "Waiting for browser…" : "Sign in with Google") {
                        Task {
                            busy = true
                            error = nil
                            error = await model.kanban.signIn()
                            busy = false
                        }
                    }
                    .disabled(busy)
                }
            }

            if let error {
                Section {
                    Label(error, systemImage: "exclamationmark.circle")
                        .foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
    }
}

/// Default terminal/editor pickers (FRONTEND.md §6.1). Changes persist
/// immediately; there is no Save button.
struct DefaultAppsSettingsView: View {
    @Environment(AppModel.self) private var model
    @State private var saveError: String?

    var body: some View {
        Form {
            Section {
                Picker("Default Terminal", selection: terminalBinding) {
                    Text("None").tag(String?.none)
                    ForEach(model.availableTerminals, id: \.id) { terminal in
                        Text(terminal.displayName).tag(Optional(terminal.id))
                    }
                }
                if model.availableTerminals.isEmpty {
                    Text("No terminal applications found on your system.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else {
                    Text("Used by a repository's “Open in …” action.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Section {
                Picker("Default Editor", selection: editorBinding) {
                    Text("None").tag(String?.none)
                    ForEach(model.availableEditors, id: \.id) { editor in
                        Text(editor.displayName).tag(Optional(editor.id))
                    }
                }
                if model.availableEditors.isEmpty {
                    Text("No code editors found on your system.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            if let saveError {
                Section {
                    Label(saveError, systemImage: "exclamationmark.circle")
                        .foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
        .task { await model.loadSettings() }
    }

    private var terminalBinding: Binding<String?> {
        Binding(
            get: { model.defaultTerminalId },
            set: { saveError = model.setDefaultTerminal($0) }
        )
    }

    private var editorBinding: Binding<String?> {
        Binding(
            get: { model.defaultEditorId },
            set: { saveError = model.setDefaultEditor($0) }
        )
    }
}

/// Git-clean exclude pattern management (FRONTEND.md §6.2).
struct GitCleanSettingsView: View {
    @Environment(AppModel.self) private var model
    @State private var newPattern = ""
    @State private var error: String?

    var body: some View {
        Form {
            Section("About Git Clean") {
                Text("Clean removes git-ignored files from a repository — build artifacts, caches, and other generated files (`git clean -fdX`).")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Section("Exclude Patterns") {
                Text("Files matching these patterns are preserved during cleaning. Use glob patterns like `.env*` or `*.key`.")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                if model.gitCleanPatterns.isEmpty {
                    Text("No exclude patterns configured. All ignored files will be removed.")
                        .font(.callout)
                        .foregroundStyle(.tertiary)
                }

                ForEach(model.gitCleanPatterns, id: \.self) { pattern in
                    HStack {
                        Text(pattern).monospaced()
                        Spacer()
                        Button(role: .destructive) {
                            error = model.removeGitCleanPattern(pattern)
                        } label: {
                            Image(systemName: "trash")
                        }
                        .buttonStyle(.borderless)
                        .help("Remove pattern")
                    }
                }

                HStack(spacing: 6) {
                    TextField("e.g. .env*, *.key, .vscode/", text: $newPattern)
                        .monospaced()
                        .onSubmit(addPattern)
                    Button("Add", action: addPattern)
                        .disabled(newPattern.trimmingCharacters(in: .whitespaces).isEmpty)
                }
            }

            if let error {
                Section {
                    Label(error, systemImage: "exclamationmark.circle")
                        .foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
        .task { model.loadGitCleanPatterns() }
    }

    private func addPattern() {
        let pattern = newPattern.trimmingCharacters(in: .whitespaces)
        guard !pattern.isEmpty else { return }
        error = model.addGitCleanPattern(pattern)
        if error == nil { newPattern = "" }
    }
}
