import SwiftUI

/// GitHub-repo-backed kanban board (FRONTEND.md §7). Cards auto-populate
/// from the user's GitHub repositories; drag between columns to organize.
struct KanbanBoardView: View {
    @Environment(AppModel.self) private var model

    private var kanban: KanbanModel { model.kanban }

    /// Minimum width before the board scrolls horizontally instead of
    /// compressing: 5 columns × 224 + 4 gaps × 12 + 2 × 12 padding.
    private static let minBoardWidth: CGFloat = 5 * 224 + 4 * 12 + 24

    var body: some View {
        Group {
            if kanban.isLoading {
                ProgressView("Loading board…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let auth = kanban.auth, kanban.authedGhUser == nil {
                GhUnavailableView(auth: auth)
            } else {
                board
            }
        }
        .task { kanban.startIfNeeded() }
        .navigationTitle("Kanban")
    }

    private var board: some View {
        let entries = kanban.board(matching: model.searchText)
        return ScrollView(.horizontal) {
            HStack(alignment: .top, spacing: 12) {
                ForEach(KanbanColumn.allCases) { column in
                    KanbanColumnView(column: column, entries: entries[column] ?? [])
                }
            }
            .padding(12)
            .containerRelativeFrame(.horizontal, alignment: .leading) { length, _ in
                max(length, Self.minBoardWidth)
            }
        }
        .scrollBounceBehavior(.basedOnSize, axes: .horizontal)
        .safeAreaInset(edge: .top, spacing: 0) { boardHeader }
    }

    @ViewBuilder
    private var boardHeader: some View {
        if kanban.errorMessage != nil || kanban.authedGhUser != nil {
            HStack(spacing: 8) {
                if let error = kanban.errorMessage {
                    Label(error, systemImage: "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundStyle(.red)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                Spacer(minLength: 0)
                if let user = kanban.authedGhUser {
                    // gh can report an empty account name; match the Tauri
                    // fallback rather than rendering a dangling "gh: ".
                    Text("gh: \(user.isEmpty ? "authenticated" : user)")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 5)
        }
    }
}

/// One column: tinted header (dot + title + count capsule) over a scrolling
/// card list; a drop target for card moves.
struct KanbanColumnView: View {
    @Environment(AppModel.self) private var model
    let column: KanbanColumn
    let entries: [KanbanEntry]

    @State private var isTargeted = false

    var body: some View {
        VStack(spacing: 0) {
            header
            ScrollView {
                LazyVStack(spacing: 8) {
                    if entries.isEmpty {
                        Text("No projects")
                            .font(.caption)
                            .foregroundStyle(.tertiary)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 24)
                    } else {
                        ForEach(entries) { entry in
                            KanbanCardView(entry: entry)
                        }
                    }
                }
                .padding(10)
            }
        }
        .frame(minWidth: 224, maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(isTargeted ? AnyShapeStyle(column.color.opacity(0.08)) : AnyShapeStyle(.quinary))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .strokeBorder(
                    isTargeted ? AnyShapeStyle(column.color.opacity(0.7)) : AnyShapeStyle(.separator),
                    lineWidth: isTargeted ? 2 : 1
                )
        )
        .dropDestination(for: String.self) { items, _ in
            guard let nameWithOwner = items.first else { return false }
            model.kanban.move(nameWithOwner, to: column)
            return true
        } isTargeted: { isTargeted = $0 }
        .animation(.easeOut(duration: 0.15), value: isTargeted)
    }

    private var header: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(column.color)
                .frame(width: 8, height: 8)
            Text(column.title)
                .font(.subheadline.weight(.semibold))
                .lineLimit(1)
            Spacer(minLength: 4)
            Text("\(entries.count)")
                .font(.caption.weight(.semibold))
                .foregroundStyle(column.color)
                .padding(.horizontal, 7)
                .padding(.vertical, 2)
                .background(column.color.opacity(0.13), in: Capsule())
        }
        .padding(.horizontal, 12)
        .padding(.top, 10)
        .padding(.bottom, 8)
    }
}

/// One repo card: name + archived badge, owner + private lock, relative
/// pushed time, and the notes row when the card has notes, edited in place
/// (a click on the notes or a double-click on the card opens the editor).
/// Draggable by nameWithOwner except while its notes are being edited;
/// actions on hover/right-click.
struct KanbanCardView: View {
    @Environment(AppModel.self) private var model
    let entry: KanbanEntry

    @State private var isHovering = false
    @State private var confirmDelete = false
    /// The notes the editor opened with; nil while not editing. The save
    /// compares against this rather than the card, because a focus refresh
    /// can land a remote edit on the card while the editor is open, and an
    /// untouched draft must not overwrite that.
    @State private var notesOpenedWith: String?
    @State private var notesDraft = ""
    @State private var notesDiscarded = false
    @FocusState private var isNotesFocused: Bool

    private var isEditingNotes: Bool { notesOpenedWith != nil }

    private var canDelete: Bool {
        guard let user = model.kanban.authedGhUser else { return false }
        return user.lowercased() == entry.repo.owner.login.lowercased()
    }

    private var notesActionTitle: String {
        entry.card.notes == nil ? "Add Notes…" : "Edit Notes…"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(spacing: 6) {
                Text(entry.repo.name)
                    .font(.callout.weight(.medium))
                    .lineLimit(1)
                if entry.repo.isArchived {
                    Text("ARCHIVED")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(.yellow)
                }
                Spacer(minLength: 0)
                if isHovering {
                    Menu {
                        cardActions
                    } label: {
                        Image(systemName: "ellipsis.circle")
                    }
                    .menuIndicator(.hidden)
                    .buttonStyle(.borderless)
                    .fixedSize()
                }
            }
            .frame(minHeight: 18)

            HStack(spacing: 5) {
                Text(entry.repo.owner.login)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                if entry.repo.isPrivate {
                    Image(systemName: "lock.fill")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                        .help("Private")
                }
                Spacer(minLength: 4)
                if let pushed = entry.pushedDate {
                    Text(pushed, format: .relative(presentation: .named))
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                }
            }

            notesRow
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 10).fill(.regularMaterial))
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .strokeBorder(.separator, lineWidth: 1)
        )
        .shadow(color: .black.opacity(isHovering ? 0.12 : 0.05), radius: isHovering ? 3 : 1, y: 1)
        // Double-click opens the editor, the mouse entry point for a card
        // without notes. Simultaneous, so the notes text's own tap does not
        // wait for a possible second click. A double-click whose first click
        // closed the editor does not reopen it: closing flips the drag switch
        // below, which rebuilds the card and discards that first click.
        .simultaneousGesture(TapGesture(count: 2).onEnded { startEditingNotes() })
        // Off while editing. Flipping it rebuilds the card below this line;
        // the editor's state lives on this view and survives.
        .draggableIf(!isEditingNotes, entry.repo.nameWithOwner)
        .onHover { isHovering = $0 }
        .contextMenu { cardActions }
        .help(entry.repo.description ?? entry.repo.nameWithOwner)
        .alert("Delete repository?", isPresented: $confirmDelete) {
            Button("Delete", role: .destructive) {
                Task { await model.kanban.deleteRepo(entry.repo.nameWithOwner) }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text(
                """
                This will permanently delete the GitHub repository \
                “\(entry.repo.nameWithOwner)”.

                This cannot be undone.
                """
            )
        }
        // Focus leaving the editor, by a click elsewhere or by Return or
        // Escape in `notesRow`, is the one way out of it. This sits on the
        // card, which outlives the editor, so it survives the rebuild that
        // closing causes.
        .onChange(of: isNotesFocused) { _, focused in
            if !focused { finishEditingNotes(save: !notesDiscarded) }
        }
    }

    private var notesBoxShape: some InsettableShape {
        RoundedRectangle(cornerRadius: 5)
    }

    /// The fill behind the notes in both states, padded outward so the text
    /// inside sits where the owner row's text does.
    private var notesBox: some View {
        notesBoxShape.fill(.quinary).padding(-4)
    }

    /// Third row: the editor while editing, the notes when there are any,
    /// nothing otherwise, so a card without notes keeps its height. Both
    /// states share the box, so opening the editor moves nothing.
    @ViewBuilder
    private var notesRow: some View {
        if isEditingNotes {
            TextField("Add notes…", text: $notesDraft, axis: .vertical)
                .lineLimit(1...5)
                .textFieldStyle(.plain)
                .font(.caption)
                .focused($isNotesFocused)
                .background(notesBox)
                // `.plain` draws no focus ring; the outline is what tells the
                // editor from the notes it replaced.
                .overlay(notesBoxShape.strokeBorder(Color.accentColor.opacity(0.4)).padding(-4))
                // A focus request made synchronously as the field appears is
                // dropped; the async hop of `task` lands after it is installed.
                .task { isNotesFocused = true }
                // Return and Escape just drop focus, Escape marking the
                // draft as discarded first; the card's focus handler closes.
                .onSubmit { isNotesFocused = false }
                .onExitCommand {
                    notesDiscarded = true
                    isNotesFocused = false
                }
                // AppKit keeps the field focused when a click lands on nothing
                // focusable; this drops it, so any click outside the field
                // closes the editor, as it does in the browser.
                .endsFocusOnClickOutside($isNotesFocused)
        } else if let notes = entry.card.notes {
            Text(notes)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(3)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)
                // After the frame, or the box would hug the text.
                .background(notesBox)
                .contentShape(Rectangle())
                // High priority so the click reaches the text before the
                // card's drag handling can claim it.
                .highPriorityGesture(TapGesture().onEnded { startEditingNotes() })
        }
    }

    private func startEditingNotes() {
        // Choosing the menu item while the editor is open keeps the draft.
        guard !isEditingNotes else { return }
        let current = entry.card.notes ?? ""
        notesDraft = current
        notesDiscarded = false
        notesOpenedWith = current
    }

    /// Reached when the editor's focus drops. Saves unless Escape marked the
    /// draft as discarded, and only a draft that differs from what the
    /// editor opened with. Harmless when the editor is already closed.
    private func finishEditingNotes(save: Bool) {
        guard let openedWith = notesOpenedWith else { return }
        notesOpenedWith = nil
        guard save, notesDraft != openedWith else { return }
        model.kanban.setNotes(entry.repo.nameWithOwner, notesDraft)
    }

    @ViewBuilder
    private var cardActions: some View {
        Button(notesActionTitle, action: startEditingNotes)
        Button("View on GitHub") { model.kanban.openOnGitHub(entry.repo) }
        if canDelete {
            Divider()
            Button("Delete Repository…", role: .destructive) { confirmDelete = true }
        }
    }
}

/// Empty state when the GitHub CLI is missing, unauthenticated, or erroring
/// (the board is unusable without it).
struct GhUnavailableView: View {
    @Environment(AppModel.self) private var model
    let auth: GhAuthStatus

    var body: some View {
        ContentUnavailableView {
            Label(headline, systemImage: "terminal")
        } description: {
            Text(descriptionText)
        } actions: {
            if case .notInstalled = auth {
                Button("Open cli.github.com") {
                    model.kanban.open(url: "https://cli.github.com")
                }
            }
            Button("Recheck") {
                Task { await model.kanban.refresh() }
            }
            .buttonStyle(.glassProminent)
        }
        .navigationTitle("Kanban")
    }

    private var headline: String {
        switch auth {
        case .notInstalled: "GitHub CLI Not Found"
        case .notAuthenticated: "GitHub CLI Not Authenticated"
        default: "GitHub CLI Error"
        }
    }

    private var descriptionText: String {
        switch auth {
        case .notInstalled:
            "The kanban board is built from your GitHub repositories. Install gh, then recheck."
        case .notAuthenticated:
            "Run “gh auth login” in a terminal, then recheck."
        case .error(let message):
            message
        case .ok:
            ""
        }
    }
}
