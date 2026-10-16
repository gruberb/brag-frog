import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useRef } from 'react';
import type { PartialBlock } from '@blocknote/core';
import { filterSuggestionItems, SuggestionMenu } from '@blocknote/core/extensions';
import { SuggestionMenuController, useCreateBlockNote, type DefaultReactSuggestionItem } from '@blocknote/react';
import { BlockNoteView } from '@blocknote/mantine';
import '@blocknote/mantine/style.css';
import './editor.css';
import { bragSchema } from './schema';
import { EntryRefContext } from './entry-ref-context';
import { useAutoSave } from '@/hooks/use-auto-save';
import type { EntryRefCandidate, RichEditorHandle } from './types';

type Props = {
  /** Identity of the document; the editor remounts when it changes. */
  docKey: string;
  initialBlocks: PartialBlock[];
  editable?: boolean;
  /** Called after the 500 ms debounce, on unmount and on page hide. */
  onSave: (blocks: PartialBlock[]) => Promise<unknown>;
  onSaveError?: (err: unknown) => void;
  searchEntries?: (query: string) => Promise<EntryRefCandidate[]>;
  onOpenEntry?: (id: number) => void;
  onDirty?: () => void;
  placeholder?: string;
};

const SEARCH_DEBOUNCE_MS = 150;

const RichEditor = forwardRef<RichEditorHandle, Props>(function RichEditor(
  { docKey, initialBlocks, editable = true, onSave, onSaveError, searchEntries, onOpenEntry, onDirty, placeholder = 'Type / for blocks, or [[ to reference an entry.' },
  ref,
) {
  const editor = useCreateBlockNote(
    { schema: bragSchema, initialContent: initialBlocks.length ? (initialBlocks as never) : undefined, placeholders: { default: placeholder } },
    [docKey],
  );
  const { schedule, flush } = useAutoSave<PartialBlock[]>(onSave, { onError: onSaveError });
  const applyingExternal = useRef(false);
  const wrapper = useRef<HTMLDivElement>(null);

  const insertEntryRef = useCallback(
    (c: EntryRefCandidate) => {
      (editor as unknown as { insertInlineContent: (c: unknown[]) => void }).insertInlineContent([{ type: 'entryRef', props: { entryId: String(c.id), label: c.title } }, ' ']);
      editor.focus();
    },
    [editor],
  );

  useImperativeHandle(
    ref,
    () => ({
      insertEntryRef,
      insertEvidence: (item) => {
        // A bullet with the title (a link when the item has a source URL) and a muted source/date suffix.
        const content = [
          item.url ? { type: 'link', href: item.url, content: item.title } : { type: 'text', text: item.title, styles: {} },
          ...(item.meta ? [{ type: 'text', text: ` (${item.meta})`, styles: {} }] : []),
        ];
        const at = editor.getTextCursorPosition().block;
        const empty = Array.isArray(at.content) && at.content.length === 0 && at.type === 'paragraph';
        const block = { type: 'bulletListItem', content } as never;
        if (empty) editor.replaceBlocks([at], [block]);
        else editor.insertBlocks([block], at, 'after');
        editor.focus();
      },
      replaceAll: (blocks) => {
        applyingExternal.current = true;
        try {
          editor.replaceBlocks(editor.document, blocks as never);
        } finally {
          applyingExternal.current = false;
        }
        // An AI draft replaces content on purpose, so it is saved like a user edit.
        schedule(editor.document as PartialBlock[]);
        onDirty?.();
      },
      insertBlocks: (blocks) => {
        const last = editor.document[editor.document.length - 1];
        editor.insertBlocks(blocks as never, last, 'after');
        schedule(editor.document as PartialBlock[]);
        onDirty?.();
      },
      getBlocks: () => editor.document as PartialBlock[],
      isEmpty: () => editor.document.length === 0 || (editor.document.length === 1 && Array.isArray(editor.document[0].content) && editor.document[0].content.length === 0),
      flush,
    }),
    [editor, insertEntryRef, schedule, flush, onDirty],
  );

  const onChange = useCallback(() => {
    if (applyingExternal.current) return;
    schedule(editor.document as PartialBlock[]);
    onDirty?.();
  }, [editor, schedule, onDirty]);

  // BlockNote misses `[[` typed mid-text, so the second bracket is detected on the input event
  // (mobile keyboards do not fire reliable keydown for brackets) and the menu is opened by hand.
  useEffect(() => {
    const el = wrapper.current;
    if (!el) return;
    let t: ReturnType<typeof setTimeout> | null = null;
    const onInput = (e: Event) => {
      const data = (e as InputEvent).data;
      if (data && !data.includes('[')) return;
      t = setTimeout(() => {
        const menu = editor.getExtension(SuggestionMenu);
        if (!menu || menu.shown()) return;
        const view = editor.prosemirrorView;
        if (!view) return;
        const { from } = view.state.selection;
        if (from < 2) return;
        try {
          if (view.state.doc.textBetween(from - 2, from) === '[[') {
            view.dispatch(view.state.tr.delete(from - 2, from));
            menu.openSuggestionMenu('[[', { deleteTriggerCharacter: true });
          }
        } catch {
          // textBetween throws near node boundaries; ignore.
        }
      }, 0);
    };
    el.addEventListener('input', onInput);
    return () => {
      el.removeEventListener('input', onInput);
      if (t) clearTimeout(t);
    };
  }, [editor]);

  const latestSearch = useRef(0);
  const refItems = useCallback(
    async (query: string): Promise<(DefaultReactSuggestionItem & { candidate?: EntryRefCandidate })[]> => {
      if (!searchEntries) return [];
      const call = ++latestSearch.current;
      if (query.trim()) {
        await new Promise((r) => setTimeout(r, SEARCH_DEBOUNCE_MS));
        // The menu drops stale results, so an empty list for a superseded query is safe.
        if (call !== latestSearch.current) return [];
      }
      const found = await searchEntries(query);
      return found.map((c) => ({ title: c.title.length > 60 ? `${c.title.slice(0, 57)}...` : c.title, subtext: c.subtext, candidate: c, onItemClick: () => {} }));
    },
    [searchEntries],
  );

  const slashItems = useMemo(
    () => async (query: string) => filterSuggestionItems((await import('@blocknote/react')).getDefaultReactSlashMenuItems(editor), query),
    [editor],
  );

  return (
    <EntryRefContext.Provider value={onOpenEntry ?? (() => {})}>
      <div ref={wrapper} className="editor-host">
        <BlockNoteView editor={editor} editable={editable} theme="light" onChange={onChange} slashMenu={false}>
          <SuggestionMenuController triggerCharacter="/" getItems={slashItems} />
          {searchEntries && (
            <SuggestionMenuController
              triggerCharacter="[["
              getItems={refItems as never}
              onItemClick={(item: { candidate?: EntryRefCandidate }) => item.candidate && insertEntryRef(item.candidate)}
            />
          )}
        </BlockNoteView>
      </div>
    </EntryRefContext.Provider>
  );
});

export default RichEditor;
