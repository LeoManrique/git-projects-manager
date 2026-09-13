# Plan: bring back per-card notes on the Kanban board

Status: in progress. Slices 1 to 3 are done (core model, store, service,
server, tests). Next: slice 4, the Tauri plumbing.

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
  -> hook / view model: optimistic state, then call core
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

### 5. Tauri command: `desktop/src-tauri/`

- `src/commands/kanban.rs`: `#[tauri::command] pub async fn update_kanban_notes(state, name_with_owner: String, notes: Option<String>) -> Result<KanbanState, String>`
  mirroring `move_kanban_card`.
- `src/main.rs`: register it in `generate_handler!` after `move_kanban_card`.

### 6. Desktop TypeScript: `desktop/src/`

- `types/kanban.ts`: add `notes?: string` to `KanbanCard` (optional, since
  the JSON omits it when empty).
- `lib/api.ts`: `updateKanbanNotes(nameWithOwner: string, notes: string | null): Promise<KanbanState>`
  invoking `update_kanban_notes` with `{ nameWithOwner, notes }`.
- `hooks/useKanban.ts`: add `updateNotes` to `UseKanbanReturn` and implement
  it like `moveCard`: optimistic `setState` writing `notes` and `updatedAt`,
  then `api.updateKanbanNotes`, `setState(newState)`, and on error `setError`
  plus `doRefresh()`. Undefined instead of null in the optimistic state when
  clearing, so the shape matches what core returns.
- `components/kanban/KanbanBoard.tsx` and `KanbanColumn.tsx`: thread an
  `onUpdateNotes(nameWithOwner, notes)` prop down to the card, the same way
  `onDeleteRepo` is threaded.

### 7. Desktop card UI: `desktop/src/components/kanban/KanbanCard.tsx`

- State: `isEditing` and `draft`.
- Root `div`: `draggable={!menu.isOpen && !isEditing}`; drop the `cursor-grab`
  classes while editing.
- Notes row after the owner row: when not editing and notes exist, a `button`
  with `text-left text-[11px] text-text-muted line-clamp-3 whitespace-pre-wrap break-words`
  that enters edit mode. When editing, a `textarea` with `autoFocus`,
  `fieldSizing: 'content'`, `max-h` around 5 lines, `resize-none`,
  `onMouseDown={(e) => e.stopPropagation()}` so a drag never starts from it,
  `onBlur` saves, `onKeyDown` handles Cmd/Ctrl+Enter and Escape.
- Save path: trim, compare with `card.notes ?? ''`, call `onUpdateNotes` only
  when changed, pass `null` for empty.
- Dropdown menu: add *Add Notes…* / *Edit Notes…* above *View on GitHub*. It
  closes the menu and enters edit mode.
- There is no shared textarea component in `ui/`. The editor stays local to
  the card unless a second use appears.

### 8. UniFFI bridge: `macos/ffi/src/lib.rs`

- `KanbanCard` record: add `pub notes: Option<String>` and map it in the
  `From<domain::kanban::KanbanCard>` impl. UniFFI turns it into `String?`.
- Add `pub async fn update_kanban_notes(&self, name_with_owner: String, notes: Option<String>) -> FfiResult<KanbanState>`
  next to `move_kanban_card`, with the same `needless_pass_by_value` allow.
- Regenerate the Swift bindings with `just macos-project`.

### 9. macOS app: `macos/GitProjectsManager/`

- `Models/KanbanModel.swift`: `func setNotes(_ nameWithOwner: String, _ notes: String?)`
  mirroring `move(_:to:)`: normalize (trim, empty becomes nil), no-op when
  unchanged, optimistic `state` mutation with `updatedAt`, then
  `core.updateKanbanNotes`, on error `errorMessage` and `refresh()`.
- `Views/KanbanBoardView.swift`, `KanbanCardView`:
  - `@State isEditingNotes`, `@State draft`, `@FocusState isNotesFocused`.
  - Notes row under the owner row: `Text(notes)` with `.lineLimit(3)`,
    secondary color, `.onTapGesture` enters edit mode. While editing,
    `TextField("Add notes…", text: $draft, axis: .vertical)` with
    `.lineLimit(1...5)`, `.textFieldStyle(.plain)`, focused on appear,
    `.onSubmit` saves (Return), `.onExitCommand` cancels (Escape), and
    `.onChange(of: isNotesFocused)` saves when focus leaves.
  - `.draggable` cannot be switched off by a value. Add a small
    `draggableIf(_ enabled: Bool, _ payload: String)` view extension in
    `Views/` that applies `.draggable` only when enabled, and use it with
    `!isEditingNotes`.
  - `cardActions` gains *Add Notes…* / *Edit Notes…*. Both the hover `Menu`
    and `.contextMenu` reuse it, so both entry points come for free.
  - Search in `KanbanModel.board(matching:)` stays on `owner/name` only.

### 10. Docs

- `FRONTEND.md` section 7: extend the **Card** bullet with the notes row and
  add a **Notes** bullet with the behavior above. Section 8 non-goals: add
  "notes are not searchable". Section 9 platform table if it lists card
  behaviors.
- `DESIGN.md`: mention notes in the Kanban paragraph.
- `TECHNICAL.md`: no file-version change. Mention the `notes` column and the
  startup `ALTER TABLE` guard in the server line if the schema gets
  documented there.
- `ROADMAP.md`: a Done entry.
- `desktop/docs/DESIGN.md` still describes the v1 notes and `kanban.json`.
  Refresh that section instead of leaving the stale text.
- Delete this plan once shipped, or move it to a done section if the folder
  keeps history.

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
4. **Tauri plumbing.** Command, handler registration, TS type, API wrapper,
   hook, prop threading. Gate: `just check-desktop`.
5. **Tauri card UI.** Notes row, textarea, keyboard handling, drag guard,
   menu item. Gate: `just check-desktop`, manual test in `just dev`.
6. **UniFFI bridge.** Record field, `From`, export, regenerated bindings.
   Gate: `just macos-project`, `just test`, `just clippy`.
7. **macOS model and card UI.** `setNotes`, `draggableIf`, notes row, editor,
   menu item. Gate: `just dev-macos`, manual test.
8. **Docs.** The five documents listed above. Gate: read-through.

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
   *Add Notes…*, type two lines, press Cmd+Enter. The card shows the text.
2. Click the text, change it, click elsewhere. The change sticks.
3. Click the text, type something, press Escape. The previous text is back.
4. Clear the text completely and blur. The notes row disappears.
5. While editing, try to drag the card. Nothing happens. After saving, drag
   it to another column. It moves.
6. Sign in, add notes on a card, then open the macOS app and press Refresh.
   The notes appear on the same card.
7. macOS: edit them, press Return. Option+Return inserts a line break.
   Escape cancels. Right-click shows *Edit Notes…*.
8. Back in the Tauri app, Refresh. The macOS edit appears.
9. Quit both apps, open `kanban_v2.json`. Cards without notes have no
   `notes` key, edited ones do, and `version` is still 2.
