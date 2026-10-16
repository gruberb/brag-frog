import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Drawer } from '@/components/ui/Drawer';
import { ErrorState } from '@/components/ui/EmptyState';
import { Select } from '@/components/ui/Select';
import { SkeletonRows } from '@/components/ui/Spinner';
import { usePriorityOptions } from '@/hooks/use-priority-options';
import { useCycleContext, useSession } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { PrepResponse } from '@/types/api/models';
import { useDraftPrep, useMeetingPrep, useSavePrep } from '../api/today';

type Props = { meeting: { id: number; title: string; start_time: string | null } | null; onClose: () => void };

type Draft = { notes: string; docs: string; goal: string; priority: string };
const fromPrep = (p: PrepResponse): Draft => ({ notes: p.note?.notes ?? '', docs: p.note?.doc_urls ?? '', goal: p.note?.meeting_goal ?? '', priority: p.entry.priority_id ? String(p.entry.priority_id) : '' });

function PrepForm({ prep, entryId, onClose }: { prep: PrepResponse; entryId: number; onClose: () => void }) {
  const { cycle, readOnly } = useCycleContext();
  const { has_ai } = useSession();
  const { priorities } = usePriorityOptions(cycle?.id);
  const save = useSavePrep();
  const draftAi = useDraftPrep();
  const [d, setD] = useState<Draft>(fromPrep(prep));
  const set = (k: keyof Draft) => (e: { target: { value: string } }) => setD((x) => ({ ...x, [k]: e.target.value }));

  return (
    <form
      className="col"
      style={{ gap: 14 }}
      onSubmit={(e) => {
        e.preventDefault();
        save.mutate(
          { entryId, prep: { notes: d.notes, doc_urls: d.docs.split('\n').map((s) => s.trim()).filter(Boolean).join('\n'), meeting_goal: d.goal, priority_id: d.priority ? Number(d.priority) : undefined, clear_priority: !d.priority } },
          { onSuccess: () => { toast('Prep saved'); onClose(); }, onError: toastError },
        );
      }}
    >
      <fieldset disabled={readOnly} style={{ display: 'contents' }}>
        <div className="field"><label htmlFor="mp-goal">Goal of this meeting</label><input id="mp-goal" value={d.goal} onChange={set('goal')} /></div>
        <div className="field"><label htmlFor="mp-prio">Priority</label><Select id="mp-prio" value={d.priority} onChange={set('priority')} options={priorities} placeholder="No priority" /></div>
        <div className="field"><label htmlFor="mp-notes">Notes</label><textarea id="mp-notes" rows={7} value={d.notes} onChange={set('notes')} /></div>
        <div className="field"><label htmlFor="mp-docs">Docs (one URL per line)</label><textarea id="mp-docs" rows={3} value={d.docs} onChange={set('docs')} /></div>
      </fieldset>
      <div className="row between">
        {has_ai && !readOnly ? (
          <Button
            disabled={draftAi.isPending}
            onClick={() => draftAi.mutate({ entryId, meetingGoal: d.goal }, { onSuccess: (r) => setD((x) => ({ ...x, notes: x.notes ? `${x.notes}\n\n${r.content}` : r.content })), onError: toastError })}
          >
            {draftAi.isPending ? 'Drafting…' : 'Draft with AI'}
          </Button>
        ) : <span />}
        {!readOnly && <Button variant="primary" type="submit" disabled={save.isPending}>Save</Button>}
      </div>
    </form>
  );
}

export function MeetingPrepDrawer({ meeting, onClose }: Props) {
  const prep = useMeetingPrep(meeting?.id ?? null);
  return (
    <Drawer open={meeting != null} onOpenChange={(o) => !o && onClose()} title={meeting?.title ?? 'Meeting'} subtitle={meeting?.start_time ? `Prep · ${meeting.start_time}` : 'Prep'}>
      {prep.isPending && <SkeletonRows rows={3} h={44} />}
      {prep.error && <ErrorState error={prep.error} onRetry={() => prep.refetch()} />}
      {/* Keyed by meeting so the form state resets when another meeting opens. */}
      {prep.data && meeting && <PrepForm key={meeting.id} prep={prep.data} entryId={meeting.id} onClose={onClose} />}
    </Drawer>
  );
}
