# Plan: bring back per-card notes on the Kanban board

Status: implemented end to end in both apps, docs included. What remains is
the manual test script at the bottom; delete this file once it passes.

## Background

The first Kanban implementation (commit `fef964a`, January 2026) had a free-text
`notes` field on every card, edited inline on the card. Commit `f499cba` (May 9,
2026, "Integrate GitHub CLI as kanban data source") removed it on purpose
together with manual card removal: card identity moved from the local path to
GitHub `nameWithOwner`, the store started fresh in `kanban_v2.json`, and the
notes were dropped instead of migrated. Nothing has reintroduced them since,
in either app.

The old v1 file survived because the legacy cleanup in
`core/src/infrastructure/kanban_store.rs` looks for `kanban.json` inside
`~/Library/Application Support/git-projects-manager/`, while the v1 app wrote
to the sibling folder `.git-projects-manager` (leading dot). Its 17 notes are
exported to `docs/plans/kanban-notes-v1-export.txt` for manual review. The app
does not import them: the user re-enters the ones worth keeping and deletes
the export file.

## Decisions

| Question | Decision |
|---|---|
| Editor | Inline on the card, quick edit, in both apps. |
| Cloud sync | Yes. Notes travel with the card through `/v1/sync`. |
| Old notes | Exported to a text file for manual review, no in-app import. |
| Search | Notes are not searchable. Goes into FRONTEND.md non-goals. |
| File version | `kanban_v2.json` keeps `version: 2`. A missing `notes` reads as none. |
| Conflicts | Existing per-card last-writer-wins by `updatedAt`, whole card. A move on one device and a notes edit on another resolve to the newer card. Accepted. |

## User-facing behavior (goes into FRONTEND.md section 7)

- **Display.** A card with notes shows them as a third row under the owner
  row: muted color, small text, clamped to 3 lines, line breaks preserved.
  A card without notes shows nothing extra, so card heights and the compact
  board stay as they are today.
- **Entering edit mode.** Clicking the notes text edits it in place. The card
  actions menu (hover ellipsis in both apps, plus the right-click menu on
  macOS) gains an item that reads *Add Notes…* when the card has none and
  *Edit Notes…* otherwise. The menu item is the only entry point for empty
  cards, which avoids a placeholder row on every card.
- **Editing.** A multi-line text field replaces the notes row, focused, with
  placeholder "Add notes…". It grows with the content up to about 5 lines,
  then scrolls.
- **Saving.** Losing focus saves. Cmd+Enter on the Tauri app saves. Return on
  macOS saves and Option+Return inserts a line break. The text is trimmed and
  an empty result clears the notes.
- **Cancelling.** Escape restores the previous text and leaves edit mode.
- **Drag and drop.** A card in edit mode cannot be dragged. Everything else
  about dragging stays the same.
- **Optimistic update.** Like moving a card: the board shows the new text at
  once, the store write bumps `updatedAt`, a one-card background sync runs
  when signed in, and a failure refetches authoritative state.
- **Tooltip.** Unchanged, still the description.

## Data flow

```
card UI (edit, save)
  -> hook / view model: normalize, skip when unchanged, optimistic state, then call core
  -> Tauri command `update_kanban_notes` | UniFFI `update_kanban_notes`
  -> services::kanban::set_notes
       -> KanbanManager::update_notes (locked read-modify-write, bumps updatedAt)
       -> fire-and-forget one-card sync (same helper as move_card)
  -> KanbanState back to the UI
```

The server stores `notes` next to `column_id` and applies it with the same
last-writer-wins upsert. `merge_remote` in `core/src/services/kanban.rs`
needs no change because it swaps whole cards.

## Changes by layer, data model outward

### 1. Core domain: `core/src/domain/kanban.rs`

- Add to `KanbanCard`, after `column`:
  `#[serde(default, skip_serializing_if = "Option::is_none")] pub notes: Option<String>`.
  The file stays readable by the current code and a card without notes
  serializes exactly as today.
- Add `pub fn normalize_notes(input: Option<String>) -> Option<String>`:
  trims, returns `None` for empty. One place for the rule, used by the store.

### 2. Core store: `core/src/infrastructure/kanban_store.rs`

- Add `update_notes(&self, name_with_owner: &str, notes: Option<String>) -> Result<KanbanState>`
  next to `move_card`. Both go through a private `touch_card` helper that
  applies one mutation to a card and bumps `card.updated_at`, so the
  "bump on every edit" rule lives in one place.
- `sync_with_repos` builds new cards with a struct literal: add `notes: None`.

### 3. Core service: `core/src/services/kanban.rs`

- `move_card` and `set_notes` share a private `mutate_card` helper:
  `spawn_blocking` one store call, then `sync_card_in_background(state, card)`,
  the fire-and-forget block extracted from the old `move_card`.
- `clear_expired_session` takes the token store and the auth lock instead
  of `AppState`, so the detached sync task reuses it instead of copying it.
- Add `pub async fn set_notes(state, name_with_owner, notes: Option<String>) -> Result<KanbanState>`.

### 4. Sync server: `server/` (done)

The server keeps its own copy of the card shape and has no migration
framework, only `CREATE TABLE IF NOT EXISTS` at startup.

- `server/src/db.rs`: `notes TEXT` (nullable) on `manifest_cards` for fresh
  databases, and `add_notes_column` for existing ones: it counts the column
  in `pragma_table_info('manifest_cards')` and runs
  `ALTER TABLE manifest_cards ADD COLUMN notes TEXT` only when it is missing.
  Both steps run on every start.
- `server/src/sync.rs`: `pub notes: Option<String>` on `SyncCard`, defaulted
  when missing and skipped when `None`, like the core. The upsert and the
  select live in `store_newer` and `load_all`, both carrying `notes`, with
  `notes = excluded.notes` in the `DO UPDATE SET` list.
- Overwrite semantics: `excluded.notes` fully replaces the stored value, so
  clearing notes propagates. The cost is that an older client, which sends
  cards without the field, would push `NULL`. See Rollout.
- The server is part of `just clippy` and `just test`. It is a binary crate,
  so its tests sit next to the code and share `src/test_support.rs`.

### 5. Tauri command: `desktop/src-tauri/` (done)

- `src/commands/kanban.rs`: `#[tauri::command] pub async fn update_kanban_notes(state, name_with_owner: String, notes: Option<String>) -> Result<KanbanState, String>`
  mirroring `move_kanban_card`.
- `src/main.rs`: registered in `generate_handler!` after `move_kanban_card`.

### 6. Desktop TypeScript: `desktop/src/` (done)

- `types/kanban.ts`: `notes?: string` on `KanbanCard` (optional, since the
  JSON omits it when empty).
- `lib/api.ts`: `updateKanbanNotes(nameWithOwner: string, notes: string | null): Promise<KanbanState>`
  invoking `update_kanban_notes` with `{ nameWithOwner, notes }`.
- `hooks/useKanban.ts`: `updateNotes(nameWithOwner, text)` takes the
  editor's raw text and applies the store's rule itself (`normalizeNotes`:
  trim, blank becomes undefined), so the optimistic card already has the
  shape core returns. Saving unchanged text returns early: no store write,
  no `updatedAt` bump, no sync. `moveCard` owns the matching check for
  moves (`displayedColumn`, legacy ids as Backlog), so
  `KanbanBoard.handleDrop` only hands over the dragged name. Both share a
  private `mutateCard(nameWithOwner, patch, request)`: clear the error,
  optimistic `setState` merging the patch with a provisional `updatedAt`,
  then `setState(await request())`; on failure `await doRefresh()` first and
  `setError` after, because the refresh clears the error on its way in (the
  old order lost every move error).
- `components/kanban/KanbanBoard.tsx` and `KanbanColumn.tsx` thread an
  `onUpdateNotes(nameWithOwner, text)` prop down to the card the same way
  `onDeleteRepo` is threaded; `KanbanCard.tsx` declares the prop and uses it
  in the next slice.

### 7. Desktop card UI: `desktop/src/components/kanban/KanbanCard.tsx` (done)

- State: `notesOpenedWith: string | null` (the text the editor opened with,
  null while not editing; `isEditingNotes` is derived from it) and
  `notesDraft`. One value both says "editing" and carries the untouched-draft
  reference, so they cannot disagree.
- Root `div`: `draggable={!menu.isOpen && !isEditingNotes}`; the
  `cursor-grab` classes are dropped while editing.
- Notes row after the owner row, inside the same `px-3 pb-2` wrapper as the
  owner row, rendered only while editing or when notes exist so a card
  without notes keeps its height. A `-mx-1.5 -mb-1` wrapper holds either
  the notes `div` (`cursor-text`, `line-clamp-3 whitespace-pre-wrap
  wrap-break-word`, the Tailwind 4 name; `break-words` is legacy) or the
  local `NotesEditor`. Both use the same `notesBoxClass` box (`rounded-md
  bg-dark-bg/70 px-1.5 py-1`), whose padding the wrapper pulls outward so
  the text lines up with the owner row and nothing moves when the editor
  opens; the editor adds an accent focus ring.
- Double-click: `onDoubleClick` on the card calls `startEditingNotes`, and
  the ellipsis button stops `dblclick` propagation (stopping `click` does
  not cover it). The first press of a double-click on a card being edited
  blurs and closes the editor before `dblclick` lands, so `onMouseDown`
  records `wasEditingAtPress` on first presses (`e.detail === 1`) and the
  handler skips the reopen. The card is `select-none` so a double-click
  highlights nothing; the textarea is `select-text`.
- `NotesEditor`: `field-sizing: content` plus `max-h-[calc(5lh_+_0.5rem)]`
  and `overflow-y-auto` for the growth (WebView2, Safari 26.2+ and WebKitGTK
  2.52+ support it; an older Linux WebKit ignores it and shows the `rows={3}`
  box instead, so there is no JS resize code). A mount `useLayoutEffect`
  focuses the field and selects its text, which is what a macOS field does
  when it becomes first responder, so both editors open the same way.
  Losing focus is the single exit: Cmd/Ctrl+Enter and Escape call `blur()`
  on the field, Escape setting a discard ref first, and the one `onBlur`
  handler reports `onClose(save)`. Both keys are ignored mid-composition,
  when they belong to the input method. No `onMouseDown` stopPropagation:
  the card is not draggable while editing anyway, and swallowing mousedown
  would keep another card's open menu from closing. The notes row is a
  `div`, not a `button`: the app's global button press effect (an unlayered
  `transform: scale(0.98)` no utility can override) would shrink the text
  on click.
- Save path: `finishEditingNotes(save)` calls
  `onUpdateNotes(card.nameWithOwner, notesDraft)` only when `save` and the
  draft differs from `notesOpenedWith`, then clears `notesOpenedWith`.
  Trimming, clearing on blank, and skipping a save that matches the card are
  the hook's job, so the card stays a dumb editor.
- Untouched draft: the comparison against `notesOpenedWith` (not the card)
  is what keeps a focus refresh that lands remote notes under an open editor
  from being overwritten by a plain blur.
- Dropdown menu: *Add Notes…* / *Edit Notes…* above *View on GitHub*, wired
  to the same `startEditingNotes`, which closes the menu first.
- There is no shared textarea component in `ui/`. `NotesEditor` stays in the
  card's file unless a second use appears.

### 8. UniFFI bridge: `macos/ffi/src/lib.rs` (done)

- `KanbanCard` record: `pub notes: Option<String>`, mapped in the
  `From<domain::kanban::KanbanCard>` impl. UniFFI turns it into `String?`.
- `pub async fn update_kanban_notes(&self, name_with_owner: String, notes: Option<String>) -> FfiResult<KanbanState>`
  next to `move_kanban_card`, with the same `needless_pass_by_value` allow.
- The Swift bindings are regenerated by `just macos-project`
  (`macos/generated` is gitignored, so run it after pulling this change).

### 9. macOS app: `macos/GitProjectsManager/`

- `Models/KanbanModel.swift` (done): `func setNotes(_ nameWithOwner: String, _ text: String)`
  takes the editor's raw text, like the Tauri hook: `normalizeNotes` (trim,
  blank becomes nil), no-op when unchanged. `move(_:to:)` and `setNotes` share
  a private `edit(_:change:request:)`: `change` rewrites the displayed card
  and reports whether it differed, a changed card clears `errorMessage` and
  gets a provisional `updatedAt`, then `request` calls core and its state
  replaces the optimistic one; on failure `refresh()` runs first and the
  message is set after, because `refresh()` clears it on its way in. The
  view calls `model.kanban.setNotes(nameWithOwner, draft)` on save.
- `Views/KanbanBoardView.swift`, `KanbanCardView` (done):
  - `@State notesOpenedWith: String?` (nil while not editing;
    `isEditingNotes` is derived), `@State notesDraft`,
    `@FocusState isNotesFocused`. Same shape as the Tauri card.
  - `notesRow` under the owner row: `Text(notes)` with `.lineLimit(3)`,
    secondary color, full-width `contentShape`, and a
    `.highPriorityGesture(TapGesture())` rather than `.onTapGesture`, so the
    click reaches the text before the card's drag handling can claim it
    (a plain `.gesture` is scheduled after existing ones). While editing,
    `TextField("Add notes…", text: $notesDraft, axis: .vertical)` with
    `.lineLimit(1...5)`, `.textFieldStyle(.plain)` (no bezel; both states
    share `notesBox`, a `.quinary` fill padded outward by 4 pt, and the
    editor adds an accent `strokeBorder` overlay since `.plain` draws no
    focus ring), the same single exit as the Tauri card: `.onSubmit` (Return;
    Option+Return inserts a line break) sets `isNotesFocused = false`, and
    `.onExitCommand` (Escape) sets `notesDiscarded = true` first and then
    does the same. Neither saves by itself, so the order in which AppKit
    delivers the key and the focus change cannot matter.
  - Focus is requested from `.task { isNotesFocused = true }`, not
    `.onAppear`: a synchronous request made as the field appears with its
    branch is dropped; the async hop lands after the field is installed.
    The field selects its text on focus, native macOS behavior, which the
    Tauri editor copies.
  - `.onChange(of: isNotesFocused)` sits on the card container, after
    `.alert`, because a handler on the field is torn down with it. It calls
    `finishEditingNotes(save: !notesDiscarded)`, which clears
    `notesOpenedWith` first and returns when it was already nil.
  - Untouched draft: `finishEditingNotes` calls `setNotes` only when saving
    and the draft differs from `notesOpenedWith`, the same guard as the
    Tauri card. `startEditingNotes` returns early while the editor is open
    (the hover menu and the context menu stay reachable then), so choosing
    *Edit Notes…* mid-edit keeps the draft instead of resetting it; the
    Tauri card has the same guard.
  - `.draggable` has no off switch (checked against the macOS 26 SDK:
    `DragConfiguration` carries no enable flag, and `.disabled` would also
    kill the field). `Views/DraggableIf.swift` adds
    `draggableIf<Payload: Transferable>(_ enabled: Bool, _ payload: Payload)`
    as a `@ViewBuilder` `if`, applied with `!isEditingNotes` where
    `.draggable` was. Flipping it rebuilds the subtree; the card's own
    `@State` and `@FocusState` sit above it and survive.
  - Click outside: AppKit leaves a text field first responder when a click
    hits nothing focusable, so `Views/EndFocusOnClickOutside.swift` adds
    `endsFocusOnClickOutside(_ focus: FocusState<Bool>.Binding)`, applied
    to the field: a local `NSEvent` mouse-down monitor, installed in
    `.onAppear` and removed in `.onDisappear` (both fire through the
    `draggableIf` rebuild), that walks from the window's field editor out to
    the enclosing `NSTextField` and drops the focus when the click lands
    outside its bounds or in another window, returning the event unchanged
    so the click still reaches what is under it. Setting the focus state
    from the handler lands on the next update, after the click is
    dispatched.
  - Double-click: `.simultaneousGesture(TapGesture(count: 2))` on the card
    calls `startEditingNotes`. Simultaneous rather than `.onTapGesture`,
    which would make the notes text's single tap wait about 290 ms for a
    possible second click. A double-click whose first click closed the
    editor does not reopen it: closing flips `draggableIf`, which rebuilds
    the card and discards the first click's recognition (measured with the
    gesture on either side of the switch; it holds as long as closing flips
    it, which is why this side needs no `wasEditingAtPress` latch). The
    hover `Menu` owns the mouse from its first press, so double-clicking it
    never reaches the card.
  - `cardActions` gains *Add Notes…* / *Edit Notes…* (`notesActionTitle`)
    above *View on GitHub*. Both the hover `Menu` and `.contextMenu` reuse
    it, so both entry points come for free.
  - Search in `KanbanModel.board(matching:)` stays on `owner/name` only.

### 10. Docs (done)

- `FRONTEND.md` section 7: the **Card** bullet describes the notes row and a
  **Notes** bullet carries the behavior above. Section 8 non-goals: "notes
  are not searchable". Section 9 platform table: the save keys per platform
  on the Kanban row.
- `DESIGN.md`: notes in the Kanban paragraph.
- `TECHNICAL.md`: the persistence paragraph says `notes` is optional and left
  out of the JSON when empty, so the file stays v2; the server paragraph
  already covered the column and the guarded `ALTER TABLE`.
- `ROADMAP.md`: a Done entry. `README.md`: the kanban sentence mentions notes.
- `desktop/docs/DESIGN.md`: the Kanban section now describes the current
  board (five columns, `kanban_v2.json`, notes) and points at FRONTEND.md.
- Delete this plan once the manual test script below passes.

## Tests

`core/tests/kanban_notes.rs` (done) uses the same temp-dir pattern as
`uninitialized_toggle.rs` (`std::env::temp_dir()` plus the process id), one
test function per behavior, no table-driven tests:

- `update_notes_persists_and_bumps_updated_at`
- `update_notes_trims_surrounding_whitespace`
- `update_notes_clears_when_the_text_is_blank`
- `update_notes_ignores_an_unknown_card`
- `sync_with_repos_keeps_notes_of_existing_cards`
- `a_card_without_notes_serializes_without_the_key`
- `state_without_notes_field_still_loads` (a hand-written v2 JSON with no
  `notes` key)

`merge_remote` is private, so its tests live in a unit test module inside
`core/src/services/kanban.rs` (done): `remote_notes_are_adopted_when_newer`,
`local_notes_win_when_strictly_newer`, and
`a_newer_remote_clearing_notes_wins_too`.

Server tests (done), in `server/src/db.rs`:
`a_fresh_database_gets_the_notes_column`,
`a_database_from_before_notes_gains_the_column`, and
`migrating_twice_is_harmless`. In `server/src/sync.rs`, through the handler:
`notes_are_stored_and_echoed_back`, `a_newer_card_replaces_the_notes` (which
also pins `created_at` to its first value),
`a_card_with_the_same_updated_at_changes_nothing`,
`an_older_card_leaves_the_notes_alone`,
`a_newer_card_without_notes_clears_them`,
`a_card_migrated_from_before_notes_accepts_them` (the production upgrade path
end to end), plus the two serde checks
`a_request_without_the_notes_key_still_parses` and
`a_card_without_notes_is_sent_without_the_key`.

## Slices for implementation (Code Mentor style)

Each slice builds on its own and has a proving command. Do not start the
next one before the previous is filled and reviewed.

1. **Core model and store.** (done) Field, `normalize_notes`, `update_notes`,
   `sync_with_repos` literal, the store tests. Gate: `just test`,
   `just clippy`.
2. **Core service.** (done) Shared sync helper extracted from `move_card`,
   `set_notes`, the merge tests. Gate: `just test`, `just clippy`.
3. **Server.** (done) Column plus guarded `ALTER TABLE`, `SyncCard`, upsert,
   select, server tests. Gate: `just test`, `just clippy`, then start it
   locally with `just dev-server` against a copy of an existing database to
   see the migration run.
4. **Tauri plumbing.** (done) Command, handler registration, TS type, API
   wrapper, hook, prop threading. Gate: `just check-desktop`.
5. **Tauri card UI.** (done) Notes row, `NotesEditor`, keyboard handling,
   drag guard, menu item. Gate: `just check-desktop` (passed), manual test
   in `just dev`.
6. **UniFFI bridge.** (done) Record field, `From`, export, regenerated
   bindings. Gate: `just macos-project`, `just test`, `just clippy`.
7. **macOS model and card UI.** (done) `setNotes`, `draggableIf`, notes row,
   editor, menu item. Gate: `just macos-project` then the Debug `xcodebuild`
   of `just dev-macos` (passed, no warnings), manual test.
8. **Docs.** (done) The documents listed above. Gate: read-through.

## Rollout

- Deploy the server before shipping either client. An old server ignores
  the unknown `notes` field and echoes cards without it, so a new client
  syncing against an old server would have its notes replaced by none on the
  next merge.
- Update both apps on the same machine together. They share
  `kanban_v2.json`, and a build without the field rewrites the file without
  notes on its next store write. An old client also pushes cards without
  notes to the server, which stores `NULL`.
- Back up the v1 file before touching anything:
  `cp "/Users/leo/Library/Application Support/.git-projects-manager/kanban.json" ~/Desktop/kanban-v1-backup.json`.

## Manual test script (after slice 7)

1. Tauri app: hover a card without notes, open the ellipsis menu, choose
   *Add Notes…*, type two lines, press Cmd+Enter. The card shows the text
   in a rounded box; cards without notes show no box.
2. Click the text, change it, click elsewhere. The change sticks. The
   editor's box sits exactly where the notes box was, with an accent
   outline.
3. Click the text, type something, press Escape. The previous text is back.
4. Clear the text completely and blur. The notes row disappears.
5. Double-click a card without notes. The editor opens and nothing on the
   card gets selected. Double-click the card again while editing: the
   editor closes and stays closed. Double-click the ellipsis button: the
   menu opens and closes, no editor.
6. While editing, try to drag the card. Nothing happens. After saving, drag
   it to another column. It moves.
7. Sign in, add notes on a card, then open the macOS app and press Refresh.
   The notes appear on the same card, in the same box.
8. macOS: edit them, press Return. Option+Return inserts a line break.
   Escape cancels. Right-click shows *Edit Notes…*.
9. macOS: open the editor, then click on empty board space, on another
   card, on the same card outside the field, and on a toolbar button. Each
   closes the editor (and saves a changed draft). Double-click a card
   without notes: the editor opens; a double-click while editing closes it
   without reopening.
10. Back in the Tauri app, Refresh. The macOS edit appears.
11. Quit both apps, open `kanban_v2.json`. Cards without notes have no
    `notes` key, edited ones do, and `version` is still 2.
