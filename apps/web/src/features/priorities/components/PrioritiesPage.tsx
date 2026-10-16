import { useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router';
import { Button } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import { ConfirmAction } from '@/components/ui/ConfirmAction';
import { Dialog } from '@/components/ui/Dialog';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { Seg } from '@/components/ui/Seg';
import { SkeletonRows } from '@/components/ui/Spinner';
import { paths } from '@/config/paths';
import { useCycleContext } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { Priority } from '@/types/api/models';
import { shortDate } from '@/utils/dates';
import { isDonePriority, progressPct, trackingMeta } from '@/utils/priorities';
import { useCompleteGoal, useCreateGoal, useCreatePriority, useDeleteGoal, usePriorities, useUpdateGoal } from '../api/priorities';
import { toCreateRequest, toPriorityFormValues } from '../utils/priority-form-values';
import { ImportDialog } from './ImportDialog';
import { PriorityDrawer } from './PriorityDrawer';
import { PriorityForm } from './PriorityForm';

type Filter = 'active' | 'done' | 'all';
const keep = (f: Filter) => (p: Priority) => f === 'all' || (f === 'done' ? isDonePriority(p) : !isDonePriority(p));

function Row({ p, onOpen }: { p: Priority; onOpen: () => void }) {
  const meta = trackingMeta(p.tracking_status);
  const pct = progressPct(p);
  return (
    <button type="button" className="prow" onClick={onOpen}>
      <div>
        <b style={{ fontWeight: 500 }}>{p.title}</b>
        <div className="small muted">{p.entry_count} entries{p.latest_update ? ` · last update ${shortDate(p.latest_update.created_at.slice(0, 10))}` : ''}</div>
      </div>
      {pct != null ? <div className="bar"><i style={{ width: `${pct}%`, background: meta.bar }} /></div> : <div />}
      <span className={`badge b-${meta.tone}`} style={{ justifySelf: 'start' }}>{meta.label}{p.latest_update?.is_blocker && !isDonePriority(p) ? ' · blocker' : ''}</span>
      <span className="small muted">{p.status === 'completed' && p.completed_at ? shortDate(p.completed_at.slice(0, 10)) : p.due_date ? `Due ${shortDate(p.due_date)}` : 'Ongoing'}</span>
    </button>
  );
}

export function PrioritiesPage() {
  const { cycle, readOnly } = useCycleContext();
  const q = usePriorities(cycle?.id ?? null);
  const [sp, setSp] = useSearchParams();
  const filter = (sp.get('show') as Filter) || 'active';
  const setFilter = (f: Filter) => setSp((prev) => { const n = new URLSearchParams(prev); n.set('show', f); return n; }, { replace: true });
  const [openId, setOpenId] = useState<number | null>(null);
  const [dlg, setDlg] = useState<'priority' | 'goal' | 'import' | null>(null);
  const createGoal = useCreateGoal();
  const createPriority = useCreatePriority();
  const completeGoal = useCompleteGoal();
  const updateGoal = useUpdateGoal();
  const deleteGoal = useDeleteGoal();
  const [goalTitle, setGoalTitle] = useState('');
  const [editing, setEditing] = useState<{ id: number; title: string } | null>(null);

  const data = q.data;
  const goals = useMemo(() => data?.goals.map((g) => ({ value: String(g.goal.id), label: g.goal.title })) ?? [], [data]);
  const all = useMemo(() => (data ? [...data.goals.flatMap((g) => g.priorities), ...data.unassigned] : []), [data]);
  const open = all.find((p) => p.id === openId) ?? null;
  const goalNameOf = (p: Priority | null) => data?.goals.find((g) => g.goal.id === p?.department_goal_id)?.goal.title;
  const f = keep(filter);

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Priorities</h1>
          <p>
            {cycle?.name} · {data ? `${data.goals.length} goals, ${data.counts.total} priorities` : '…'}. Cycles are managed in <Link className="link" to={paths.settings('cycles')}>Settings</Link>.
          </p>
        </div>
        <div className="row wrap">
          <Seg label="Show" value={filter} onChange={setFilter} options={[{ value: 'active', label: `Active${data ? ` (${data.counts.active + data.counts.not_started + data.counts.on_hold})` : ''}` }, { value: 'done', label: `Done${data ? ` (${data.counts.completed + data.counts.cancelled})` : ''}` }, { value: 'all', label: 'All' }]} />
          {!readOnly && <Button onClick={() => setDlg('goal')}>+ Goal</Button>}
          {!readOnly && <Button onClick={() => setDlg('import')}>Import from Lattice</Button>}
          {!readOnly && <Button variant="primary" onClick={() => setDlg('priority')}>+ Priority</Button>}
        </div>
      </div>

      {q.isPending && <SkeletonRows rows={4} h={70} />}
      {q.error && <ErrorState error={q.error} onRetry={() => q.refetch()} />}
      {data && all.length === 0 && <EmptyState title="No priorities yet">{readOnly ? 'This cycle has none.' : 'Add a goal and a priority to get started.'}</EmptyState>}
      {data?.goals.map(({ goal, priorities }) => {
        const rows = priorities.filter(f);
        if (rows.length === 0 && filter !== 'all') return null;
        return (
          <section key={goal.id} className="goal" aria-label={goal.title}>
            <div className="goal-h">
              <div><h2>{goal.title}</h2><span className="small muted">Department goal · {priorities.length} priorities{goal.status === 'completed' ? ' · completed' : ''}</span></div>
              {!readOnly && (
                <div className="row">
                  <Button size="sm" onClick={() => setEditing({ id: goal.id, title: goal.title })}>Edit</Button>
                  <ConfirmAction
                    title="Delete goal"
                    message={`Delete “${goal.title}”? Its priorities stay and move to “No goal”.`}
                    onConfirm={() => deleteGoal.mutateAsync(goal.id).then(() => toast('Goal deleted')).catch(toastError)}
                    trigger={(open) => <Button size="sm" variant="danger" onClick={open}>Delete</Button>}
                  />
                  {goal.status !== 'completed' && <Button size="sm" onClick={() => completeGoal.mutate(goal.id, { onSuccess: () => toast('Goal completed'), onError: toastError })}>Complete goal</Button>}
                </div>
              )}
            </div>
            <Card>{rows.map((p) => <Row key={p.id} p={p} onOpen={() => setOpenId(p.id)} />)}{rows.length === 0 && <div className="empty small">No priorities in this view</div>}</Card>
          </section>
        );
      })}
      {data && data.unassigned.filter(f).length > 0 && (
        <section className="goal" aria-label="No goal">
          <div className="goal-h"><h2>No goal</h2></div>
          <Card>{data.unassigned.filter(f).map((p) => <Row key={p.id} p={p} onOpen={() => setOpenId(p.id)} />)}</Card>
        </section>
      )}

      <PriorityDrawer priority={open} goalName={goalNameOf(open)} goals={goals} onClose={() => setOpenId(null)} />

      <Dialog open={dlg === 'priority'} onOpenChange={(o) => !o && setDlg(null)} title="New priority">
        <div className="card-b">
          <PriorityForm
            defaults={toPriorityFormValues()}
            goals={goals}
            submitLabel="Create priority"
            onCancel={() => setDlg(null)}
            onSubmit={async (values) => {
              try { await createPriority.mutateAsync({ ...toCreateRequest(values), cycle_id: cycle?.id }); toast('Priority created'); setDlg(null); } catch (e) { toastError(e); }
            }}
          />
        </div>
      </Dialog>
      <Dialog open={dlg === 'goal'} onOpenChange={(o) => !o && setDlg(null)} title="New department goal" width={440}>
        <form
          className="card-b col"
          onSubmit={async (e) => {
            e.preventDefault();
            if (!goalTitle.trim() || !cycle) return;
            try { await createGoal.mutateAsync({ title: goalTitle.trim(), cycle_id: cycle.id }); setGoalTitle(''); setDlg(null); toast('Goal created'); } catch (err) { toastError(err); }
          }}
        >
          <div className="field"><label htmlFor="goal-title">Title</label><input id="goal-title" value={goalTitle} onChange={(e) => setGoalTitle(e.target.value)} /></div>
          <div className="row" style={{ justifyContent: 'flex-end' }}><Button onClick={() => setDlg(null)}>Cancel</Button><Button variant="primary" type="submit" disabled={!goalTitle.trim()}>Create goal</Button></div>
        </form>
      </Dialog>
      <Dialog open={editing !== null} onOpenChange={(o) => !o && setEditing(null)} title="Edit department goal" width={440}>
        <form
          className="card-b col"
          onSubmit={async (e) => {
            e.preventDefault();
            if (!editing?.title.trim()) return;
            try { await updateGoal.mutateAsync({ id: editing.id, title: editing.title.trim() }); setEditing(null); toast('Goal updated'); } catch (err) { toastError(err); }
          }}
        >
          <div className="field"><label htmlFor="goal-edit-title">Title</label><input id="goal-edit-title" value={editing?.title ?? ''} onChange={(e) => setEditing((cur) => (cur ? { ...cur, title: e.target.value } : cur))} /></div>
          <div className="row" style={{ justifyContent: 'flex-end' }}><Button onClick={() => setEditing(null)}>Cancel</Button><Button variant="primary" type="submit" disabled={!editing?.title.trim()}>Save</Button></div>
        </form>
      </Dialog>
      <ImportDialog open={dlg === 'import'} onOpenChange={(o) => !o && setDlg(null)} cycleId={cycle?.id ?? null} />
    </>
  );
}
