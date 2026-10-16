import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType } from 'react';
import { Link } from 'react-router';
import type { PartialBlock } from '@blocknote/core';
import { Button } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import { Drawer } from '@/components/ui/Drawer';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { Menu } from '@/components/ui/Menu';
import { Popover } from '@/components/ui/Popover';
import { SkeletonRows } from '@/components/ui/Spinner';
import { LazyRichEditor } from '@/components/editor/LazyRichEditor';
import type { EntryRefCandidate, RichEditorHandle } from '@/components/editor/types';
import { paths } from '@/config/paths';
import { useConfig } from '@/hooks/use-config';
import { usePriorityOptions } from '@/hooks/use-priority-options';
import type { ExportFormat } from '@/lib/export';
import { useCycleContext, useSession } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { ReviewSection } from '@/types/api/models';
import { useDraftSection, useReview, useSaveSection } from '../api/review';
import { CareerLevelDrawer } from './CareerLevelDrawer';

export type EvidenceSlot = ComponentType<{ open: boolean; onOpenChange: (o: boolean) => void; onInsert: (e: EntryRefCandidate) => void }>;
export type EntryDrawerSlot = ComponentType<{ entryId: number | null; onClose: () => void }>;

type Props = {
  cycleId: number;
  section: string | undefined;
  onSectionChange: (slug: string) => void;
  readOnly: boolean;
  /** Evidence picker and entry drawer come from features/entries, composed by the route. */
  Evidence: EvidenceSlot;
  EntryDrawerView: EntryDrawerSlot;
  searchEntries: (query: string) => Promise<EntryRefCandidate[]>;
};

const FORMATS: { value: ExportFormat; label: string }[] = [
  { value: 'markdown', label: 'Markdown' },
  { value: 'pdf', label: 'PDF' },
  { value: 'docx', label: 'DOCX' },
];

function SectionEditor({ cycleId, section, readOnly, jiraBase, Evidence, EntryDrawerView, searchEntries, handleRef }: {
  cycleId: number; section: ReviewSection; readOnly: boolean; jiraBase: string | null; Evidence: EvidenceSlot; EntryDrawerView: EntryDrawerSlot;
  searchEntries: Props['searchEntries']; handleRef: React.RefObject<RichEditorHandle | null>;
}) {
  const [blocks, setBlocks] = useState<PartialBlock[] | null>(null);
  const [evidence, setEvidence] = useState(false);
  const [openEntry, setOpenEntry] = useState<number | null>(null);
  const [original, setOriginal] = useState(false);
  const onInsert = useCallback((e: EntryRefCandidate) => {
    handleRef.current?.insertEntryRef(e);
    toast('Reference inserted');
  }, [handleRef]);
  const save = useSaveSection(cycleId);
  // Remember whether this section started as old markdown; the notice goes away after the first save as blocknote.
  const [startedAsMarkdown] = useState(section.content_format === 'markdown' && !!section.content.trim());
  const converted = startedAsMarkdown && section.content_format === 'markdown';
  const [initial] = useState(section);

  useEffect(() => {
    let cancelled = false;
    // Content is parsed once per document; later cache patches from our own saves must not reset the editor.
    void import('@/components/editor/convert').then(async (m) => {
      const b = await m.contentToBlocks(initial.content, initial.content_format, jiraBase);
      if (!cancelled) setBlocks(b);
    });
    return () => { cancelled = true; };
  }, [jiraBase, initial]);

  if (!blocks) return <SkeletonRows rows={4} h={28} />;
  return (
    <>
      <Card className="editor-card">
        <LazyRichEditor
          ref={handleRef}
          docKey={`${cycleId}:${section.slug}`}
          initialBlocks={blocks}
          editable={!readOnly}
          onSave={(b) => save.mutateAsync({ slug: section.slug, content: JSON.stringify(b), content_format: 'blocknote' })}
          onSaveError={toastError}
          searchEntries={searchEntries}
          onOpenEntry={setOpenEntry}
        />
      </Card>
      <div className="row between" style={{ marginTop: 12 }}>
        {readOnly ? <span /> : <button type="button" className="linkish" onClick={() => setEvidence(true)}>＋ Add evidence from your logbook</button>}
        <span className="small muted" role="status">
          {save.isPending ? 'Saving…' : save.isError ? 'Not saved' : save.isSuccess ? 'Autosaved' : ''}
        </span>
      </div>
      {converted && (
        <div className="small muted" style={{ marginTop: 8 }}>
          Converted from your earlier text. <button type="button" className="linkish small" onClick={() => setOriginal(true)}>View original</button>
        </div>
      )}
      <Drawer open={original} onOpenChange={setOriginal} title="Original text">
        <pre className="small" style={{ whiteSpace: 'pre-wrap', margin: 0, fontFamily: 'inherit' }}>{initial.content}</pre>
      </Drawer>
      <Evidence open={evidence} onOpenChange={setEvidence} onInsert={onInsert} />
      <EntryDrawerView entryId={openEntry} onClose={() => setOpenEntry(null)} />
    </>
  );
}

export function ReviewPage({ cycleId, section: sectionSlug, onSectionChange, readOnly, Evidence, EntryDrawerView, searchEntries }: Props) {
  const { cycle } = useCycleContext();
  const { has_ai } = useSession();
  const review = useReview(cycleId);
  const cfg = useConfig();
  const { goals } = usePriorityOptions(cycleId);
  const draft = useDraftSection(cycleId);
  const handle = useRef<RichEditorHandle>(null);
  const [guidance, setGuidance] = useState(false);
  const [level, setLevel] = useState(false);
  const [picked, setPicked] = useState<number[]>([]);
  const [busyExport, setBusyExport] = useState(false);

  const sections = useMemo(() => review.data?.sections ?? [], [review.data]);
  const active = sections.find((s) => s.slug === sectionSlug) ?? sections[0];
  const jiraBase = cfg.data?.atlassian_base_url ?? null;

  const exportDoc = async (format: ExportFormat, scope: 'section' | 'all') => {
      if (!review.data) return;
      setBusyExport(true);
      try {
        handle.current?.flush();
        const chosen = (scope === 'section' && active ? [active] : sections).filter((s) => s.content.trim());
        if (chosen.length === 0) return toast('Nothing written yet to export');
        const { downloadReview } = await import('@/lib/export');
        await downloadReview(format, `Self review · ${cycle?.name ?? ''}`.trim(), chosen.map((s) => ({ title: s.title, content: s.content, content_format: s.content_format })), jiraBase);
      } catch (e) {
        toastError(e);
      } finally {
        setBusyExport(false);
      }
  };

  const runDraft = () => {
    if (!active) return;
    draft.mutate(
      { slug: active.slug, goal_ids: picked },
      {
        onSuccess: async ({ markdown }) => {
          const { markdownToBlocks } = await import('@/components/editor/convert');
          const blocks = await markdownToBlocks(markdown, jiraBase);
          // Never overwrite what the user wrote: a draft is appended unless the document is empty.
          if (handle.current?.isEmpty()) handle.current.replaceAll(blocks);
          else handle.current?.insertBlocks(blocks);
          toast('Draft added');
        },
        onError: toastError,
      },
    );
  };

  return (
    <>
      <div className="page-head">
        <div><h1>Self review</h1><p>{cycle?.name}{review.data ? ` · ${review.data.entry_count} entries` : ''}</p></div>
        <div className="row">
          <Menu
            trigger={<Button disabled={busyExport}>{busyExport ? 'Preparing…' : 'Export ⌄'}</Button>}
            items={[
              ...FORMATS.map((f) => ({ label: `This section as ${f.label}`, onSelect: () => void exportDoc(f.value, 'section') })),
              ...FORMATS.map((f) => ({ label: `Whole review as ${f.label}`, onSelect: () => void exportDoc(f.value, 'all') })),
            ]}
          />
          {!readOnly && (
            <Popover
              trigger={<Button variant="primary" disabled={!has_ai || !active} title={has_ai ? undefined : 'Add an Anthropic key in Settings to draft with AI'}>Draft with AI</Button>}
            >
              <div className="col" style={{ gap: 10 }}>
                <b className="small">Draft “{active?.title}”</b>
                {active?.focus_priorities && goals.length > 0 && (
                  <fieldset style={{ border: 0, padding: 0, margin: 0 }}>
                    <legend className="small muted">Emphasize these goals</legend>
                    {goals.map((g) => (
                      <label key={g.value} className="row small">
                        <input type="checkbox" checked={picked.includes(Number(g.value))} onChange={(e) => setPicked((p) => (e.target.checked ? [...p, Number(g.value)] : p.filter((x) => x !== Number(g.value))))} /> {g.label}
                      </label>
                    ))}
                  </fieldset>
                )}
                <div className="small muted">The draft is added below what you have written.</div>
                <Button variant="primary" size="sm" onClick={runDraft} disabled={draft.isPending}>{draft.isPending ? 'Drafting…' : 'Generate draft'}</Button>
              </div>
            </Popover>
          )}
        </div>
      </div>

      {review.isPending && <SkeletonRows rows={3} h={40} />}
      {review.error && <ErrorState error={review.error} onRetry={() => review.refetch()} />}
      {review.data && sections.length === 0 && <EmptyState title="No review sections configured" />}
      {active && (
        <>
          <div className="pills" role="tablist" aria-label="Review sections" style={{ marginBottom: 12 }}>
            {sections.map((s) => (
              <button key={s.slug} type="button" role="tab" aria-selected={s.slug === active.slug} data-state={s.slug === active.slug ? 'active' : 'inactive'} onClick={() => onSectionChange(s.slug)}>
                {s.title}{!s.content.trim() && <small>empty</small>}
              </button>
            ))}
          </div>
          <div className="hint" style={{ marginBottom: 12 }}>
            {active.question}{' '}
            <button type="button" className="linkish" aria-expanded={guidance} onClick={() => setGuidance((g) => !g)}>Guidance</button>
            {' · '}
            <button type="button" className="linkish" onClick={() => setLevel(true)}>Career level</button>
          </div>
          {guidance && (
            <Card className="card-b" style={{ marginBottom: 12 }}>
              <b>{active.form_question}</b>
              {active.form_guidance && <p style={{ margin: '8px 0 4px' }}>{active.form_guidance}</p>}
              {active.form_bullets.length > 0 && <ul style={{ margin: 0, paddingLeft: 20 }}>{active.form_bullets.map((b) => <li key={b}>{b}</li>)}</ul>}
              {active.form_tip && <p className="small muted" style={{ marginBottom: 0 }}>{active.form_tip}</p>}
            </Card>
          )}
          <SectionEditor
            key={`${cycleId}:${active.slug}`}
            cycleId={cycleId}
            section={active}
            readOnly={readOnly}
            jiraBase={jiraBase}
            Evidence={Evidence}
            EntryDrawerView={EntryDrawerView}
            searchEntries={searchEntries}
            handleRef={handle}
          />
        </>
      )}
      <div style={{ marginTop: 24 }}>
        <Link className="linkish small" to={paths.reviewArchive(cycleId)}>Old check-ins, impact stories and status updates (read-only) →</Link>
      </div>
      <CareerLevelDrawer open={level} onOpenChange={setLevel} />
    </>
  );
}
