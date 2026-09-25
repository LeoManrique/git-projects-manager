import { useLayoutEffect, useRef, useState } from 'react';
import { confirm } from '@tauri-apps/plugin-dialog';
import { KanbanCardView } from '../../types';
import { useContextMenu } from '../../hooks';
import { DotsIcon } from '../icons';
import { api } from '../../lib/api';
import { logError } from '../../lib/log';

interface KanbanCardProps {
  cardView: KanbanCardView;
  authedUser: string | null;
  onDragStart: (nameWithOwner: string) => void;
  onDragEnd: () => void;
  onUpdateNotes: (nameWithOwner: string, text: string) => void;
  onDeleteRepo: (nameWithOwner: string) => void;
}

// Named long form, matching macOS `.relative(presentation: .named)`.
function formatRelative(iso: string | null): string | null {
  if (!iso) return null;
  const ts = Date.parse(iso);
  if (Number.isNaN(ts)) return null;
  const diff = Date.now() - ts;
  const day = 86_400_000;
  const ago = (n: number, unit: string) => `${n} ${unit}${n === 1 ? '' : 's'} ago`;
  if (diff < day) return 'today';
  if (diff < 2 * day) return 'yesterday';
  if (diff < 7 * day) return ago(Math.floor(diff / day), 'day');
  if (diff < 30 * day) return ago(Math.floor(diff / (7 * day)), 'week');
  if (diff < 365 * day) return ago(Math.floor(diff / (30 * day)), 'month');
  return ago(Math.floor(diff / (365 * day)), 'year');
}

// The box the notes row and its editor share, so opening the editor moves
// nothing. Both sit in a wrapper that pulls the box out by its own padding.
const notesBoxClass = 'w-full rounded-md bg-dark-bg/70 px-1.5 py-1 text-[11px]';

export function KanbanCard({
  cardView,
  authedUser,
  onDragStart,
  onDragEnd,
  onUpdateNotes,
  onDeleteRepo,
}: KanbanCardProps) {
  const { card, repo } = cardView;
  const [isDragging, setIsDragging] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  // The notes the editor opened with, null while not editing. The save
  // compares against this rather than the card, because a focus refresh can
  // land a remote edit on the card while the editor is open, and an
  // untouched draft must not overwrite that.
  const [notesOpenedWith, setNotesOpenedWith] = useState<string | null>(null);
  const [notesDraft, setNotesDraft] = useState('');
  const isEditingNotes = notesOpenedWith !== null;
  // The first press of a double-click on a card being edited blurs the
  // editor, which closes it, so by the time dblclick lands the card looks
  // idle. Remembered at the press, so the second click does not reopen
  // what the first closed.
  const wasEditingAtPress = useRef(false);
  const pushed = formatRelative(repo.pushedAt);
  const menu = useContextMenu({ menuWidth: 180 });
  const showActions = isHovered || menu.isOpen;
  const canDelete =
    authedUser !== null &&
    authedUser.toLowerCase() === repo.owner.login.toLowerCase();

  const handleDragStart = (e: React.DragEvent) => {
    e.dataTransfer.effectAllowed = 'move';
    onDragStart(card.nameWithOwner);
    setIsDragging(true);
  };

  const handleDragEnd = () => {
    setIsDragging(false);
    onDragEnd();
  };

  const startEditingNotes = () => {
    menu.close();
    // Choosing the menu item while the editor is open keeps the draft.
    if (isEditingNotes) return;
    const current = card.notes ?? '';
    setNotesDraft(current);
    setNotesOpenedWith(current);
  };

  const finishEditingNotes = (save: boolean) => {
    if (save && notesDraft !== notesOpenedWith) onUpdateNotes(card.nameWithOwner, notesDraft);
    setNotesOpenedWith(null);
  };

  const handleView = async () => {
    menu.close();
    try {
      await api.openUrl(repo.url);
    } catch (err) {
      logError('Failed to open browser', err);
    }
  };

  const handleDelete = async () => {
    menu.close();
    const ok = await confirm(
      `This will permanently delete the GitHub repository "${repo.nameWithOwner}".\n\nThis cannot be undone.`,
      { title: 'Delete repository?', kind: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }
    );
    if (ok) onDeleteRepo(card.nameWithOwner);
  };

  return (
    <>
      <div
        draggable={!menu.isOpen && !isEditingNotes}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        onMouseDown={(e) => {
          if (e.detail === 1) wasEditingAtPress.current = isEditingNotes;
        }}
        onDoubleClick={() => {
          if (!wasEditingAtPress.current) startEditingNotes();
        }}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        title={repo.description ?? repo.nameWithOwner}
        // No text selection, so a double-click opens the editor without
        // highlighting the name; the textarea opts back in.
        className={`
          rounded-[10px] bg-dark-elevated border select-none
          transition-all duration-150
          ${isEditingNotes ? '' : 'cursor-grab active:cursor-grabbing'}
          ${
            isDragging
              ? 'opacity-50 shadow-lg border-dark-border'
              : 'border-dark-border hover:border-dark-borderStrong hover:shadow-md'
          }
        `}
      >
        <div className="px-3 pt-2 flex items-center gap-2">
          <span className="text-sm font-medium text-text-primary truncate flex-1">
            {repo.name}
          </span>
          {repo.isArchived && (
            <span className="shrink-0 text-[10px] text-accent-yellow uppercase tracking-wide">
              archived
            </span>
          )}
          <button
            ref={menu.buttonRef}
            onClick={(e) => {
              e.stopPropagation();
              menu.toggle();
              menu.buttonRef.current?.blur();
            }}
            onMouseDown={(e) => e.stopPropagation()}
            onDoubleClick={(e) => e.stopPropagation()}
            className={`
              shrink-0 -mr-1 p-1 rounded
              text-text-muted hover:text-text-primary hover:bg-dark-borderSubtle
              transition-opacity
              ${showActions ? 'opacity-100' : 'opacity-0 pointer-events-none'}
            `}
            aria-label="Card actions"
          >
            <DotsIcon />
          </button>
        </div>
        <div className="px-3 pb-2">
          <div className="flex items-center gap-1.5">
            <span className="text-[11px] text-text-secondary truncate">{repo.owner.login}</span>
            {repo.isPrivate && (
              <span className="shrink-0 text-text-muted" title="Private">
                <svg
                  className="w-3.5 h-3.5"
                  viewBox="0 0 32 32"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeMiterlimit={10}
                >
                  <rect x="7" y="14" width="18" height="14" />
                  <path d="M22,14v-4c0-3.3-2.7-6-6-6h0c-3.3,0-6,2.7-6,6v4" />
                </svg>
              </span>
            )}
            <span className="flex-1" />
            {pushed && (
              <span className="shrink-0 text-[11px] text-text-muted">{pushed}</span>
            )}
          </div>
          {/* Third row: the editor while editing, the notes when there are
              any, nothing otherwise, so a card without notes keeps its
              height. The wrapper pulls the box out by the box's own
              padding, so its text lines up with the owner row's. */}
          {(isEditingNotes || card.notes !== undefined) && (
            <div className="-mx-1.5 -mb-1">
              {isEditingNotes ? (
                <NotesEditor value={notesDraft} onChange={setNotesDraft} onClose={finishEditingNotes} />
              ) : (
                // A div rather than a button: the global button press effect
                // would shrink the text on click.
                <div
                  onClick={startEditingNotes}
                  className={`${notesBoxClass} cursor-text line-clamp-3 whitespace-pre-wrap wrap-break-word text-text-secondary`}
                >
                  {card.notes}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {menu.isOpen && menu.position && (
        <div
          ref={menu.menuRef}
          className="fixed z-50 bg-dark-surface border border-dark-border rounded shadow-lg py-1 min-w-[180px]"
          style={{ top: menu.position.top, left: menu.position.left }}
        >
          <button
            onClick={startEditingNotes}
            className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors text-text-primary"
          >
            {card.notes === undefined ? 'Add Notes…' : 'Edit Notes…'}
          </button>
          <button
            onClick={handleView}
            className="w-full text-left px-3 py-1.5 text-xs hover:bg-dark-borderSubtle transition-colors text-text-primary"
          >
            View on GitHub
          </button>
          {canDelete && (
            <>
              <div className="border-t border-dark-border my-1" />
              <button
                onClick={handleDelete}
                className="w-full text-left px-3 py-1.5 text-xs hover:bg-accent-red/15 transition-colors text-accent-red"
              >
                Delete Repository…
              </button>
            </>
          )}
        </div>
      )}
    </>
  );
}

interface NotesEditorProps {
  value: string;
  onChange: (text: string) => void;
  /** Called once when the editor is left; `save` is false after Escape. */
  onClose: (save: boolean) => void;
}

/**
 * Quick-edit textarea for a card's notes. It opens with its text selected,
 * as a macOS field does when focused, and grows with the text up to five
 * lines before scrolling (`field-sizing: content`; older Linux WebKit ignores
 * it and shows the `rows` box instead). Losing focus is the one way out:
 * Cmd/Ctrl+Enter and Escape drop focus, Escape marking the draft as
 * discarded first, so a single blur handler reports the outcome.
 */
function NotesEditor({ value, onChange, onClose }: NotesEditorProps) {
  const ref = useRef<HTMLTextAreaElement>(null);
  const discardRef = useRef(false);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.focus();
    el.select();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Mid-composition, Enter and Escape belong to the input method.
    if (e.nativeEvent.isComposing) return;
    if (e.key === 'Escape') {
      discardRef.current = true;
      e.currentTarget.blur();
    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      e.currentTarget.blur();
    }
  };

  return (
    <textarea
      ref={ref}
      rows={3}
      value={value}
      placeholder="Add notes…"
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={handleKeyDown}
      onBlur={() => onClose(!discardRef.current)}
      className={`block ${notesBoxClass} select-text resize-none field-sizing-content max-h-[calc(5lh_+_0.5rem)] overflow-y-auto text-text-primary placeholder:text-text-muted outline-none focus:ring-1 focus:ring-accent-blue/40`}
    />
  );
}
