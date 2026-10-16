import { useState } from 'react';
import { Link } from 'react-router';
import { format, parseISO } from 'date-fns';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Chip } from '@/components/ui/Chip';
import { ConfirmAction } from '@/components/ui/ConfirmAction';
import { Drawer } from '@/components/ui/Drawer';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { Select, type Option } from '@/components/ui/Select';
import { SkeletonRows } from '@/components/ui/Spinner';
import { paths } from '@/config/paths';
import { useCycleContext } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { Priority } from '@/types/api/models';
import { trackingMeta, trackingOptions } from '@/utils/priorities';
import { useDeletePriority, usePostUpdate, usePriorityUpdates, useUpdatePriority } from '../api/priorities';
import { toPriorityFormValues, toUpdateRequest } from '../utils/priority-form-values';
import { PriorityForm } from './PriorityForm';

type Props = { priority: Priority | null; goalName?: string; goals: Option[]; onClose: () => void };

function UpdateForm({ priority }: { priority: Priority }) {
  const post = usePostUpdate(priority.id);
  const [status, setStatus] = useState<string>(priority.tracking_status ?? 'on_track');
  const [measure, setMeasure] = useState(priority.measure_current != null ? String(priority.measure_current) : '');
  const [comment, setComment] = useState('');
  const [blocker, setBlocker] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <form
      className="card"
      onSubmit={(e) => {
        e.preventDefault();
        if (!comment.trim()) return setError('Say what changed');
        if (measure !== '' && Number.isNaN(Number(measure))) return setError('Measure must be a number');
        setError(null);
        post.mutate(
          { tracking_status: status, measure_value: measure === '' ? undefined : Number(measure), comment: comment.trim(), is_blocker: blocker },
          { onSuccess: () => { setComment(''); setBlocker(false); toast('Update posted'); }, onError: toastError },
        );
      }}
    >
      <div className="card-b col">
        <b className="small">Post an update</b>
        <div className="grid g2" style={{ gap: 12 }}>
          <div className="field"><label htmlFor="up-status">Status</label><Select id="up-status" value={status} onChange={(e) => setStatus(e.target.value)} options={trackingOptions} /></div>
          <div className="field"><label htmlFor="up-measure">Measure{priority.measure_type ? ` (${priority.measure_type})` : ''}</label><input id="up-measure" inputMode="decimal" value={measure} onChange={(e) => setMeasure(e.target.value)} /></div>
        </div>
        <textarea rows={3} aria-label="Update" placeholder="What changed? What do you need?" value={comment} onChange={(e) => setComment(e.target.value)} />
        {error && <div className="err-text" role="alert">{error}</div>}
        <div className="row between">
          <label className="small row"><input type="checkbox" checked={blocker} onChange={(e) => setBlocker(e.target.checked)} /> Blocker</label>
          <Button variant="primary" size="sm" type="submit" disabled={post.isPending}>Post update</Button>
        </div>
      </div>
    </form>
  );
}

export function PriorityDrawer({ priority, goalName, goals, onClose }: Props) {
  const { cycle, readOnly } = useCycleContext();
  const updates = usePriorityUpdates(priority?.id ?? null);
  const update = useUpdatePriority();
  const del = useDeletePriority();
  const meta = trackingMeta(priority?.tracking_status);
  const search = cycle && !cycle.is_active ? `&cycle=${cycle.id}` : '';

  return (
    <Drawer open={priority != null} onOpenChange={(o) => !o && onClose()} title={priority?.title ?? 'Priority'} subtitle={goalName}>
      {priority && (
        <>
          <div className="row wrap">
            <Badge tone={meta.tone}>{meta.label}</Badge>
            {priority.tier && <Chip>{priority.tier}</Chip>}
            {priority.due_date && <Chip>Due {format(parseISO(priority.due_date), 'MMM d')}</Chip>}
          </div>
          {!readOnly && <UpdateForm key={priority.id} priority={priority} />}
          <h3>Timeline</h3>
          {updates.isPending && <SkeletonRows rows={3} h={40} />}
          {updates.error && <ErrorState error={updates.error} onRetry={() => updates.refetch()} />}
          {updates.data?.length === 0 && <EmptyState title="No updates yet" />}
          <div className="tl">
            {updates.data?.map((u) => (
              <div key={u.id} className={u.is_blocker ? 'blk' : u.tracking_status === 'off_track' ? 'warn' : ''}>
                <b className="small">
                  {format(parseISO(u.created_at), 'MMM d')} · {trackingMeta(u.tracking_status).label}
                  {u.measure_value != null ? ` · ${u.measure_value}${priority.measure_type ? ` ${priority.measure_type}` : ''}` : ''}
                </b>
                <div>{u.comment}</div>
              </div>
            ))}
          </div>
          <h3>Linked entries <span className="muted small">({priority.entry_count})</span></h3>
          <div className="small">
            <Link className="link" to={`${paths.logbook}?priority_id=${priority.id}${search}`}>Open in logbook →</Link>
          </div>
          <details>
            <summary style={{ cursor: 'pointer', fontWeight: 500 }}>Edit details</summary>
            <div style={{ marginTop: 12 }}>
              <PriorityForm
                defaults={toPriorityFormValues(priority)}
                goals={goals}
                readOnly={readOnly}
                submitLabel="Save"
                onSubmit={async (values) => {
                  try {
                    await update.mutateAsync({ id: priority.id, input: toUpdateRequest(values) });
                    toast('Priority saved');
                  } catch (e) {
                    toastError(e);
                  }
                }}
              />
            </div>
          </details>
          {!readOnly && (
            <div className="row between">
              {priority.status !== 'completed' ? (
                <Button onClick={() => update.mutate({ id: priority.id, input: { status: 'completed', tracking_status: 'complete' } }, { onSuccess: () => toast('Marked as completed'), onError: toastError })}>Mark completed</Button>
              ) : <span />}
              <ConfirmAction
                title="Delete priority"
                message={`Delete “${priority.title}” and its updates? Linked entries stay in your logbook.`}
                onConfirm={async () => {
                  try { await del.mutateAsync(priority.id); onClose(); } catch (e) { toastError(e); }
                }}
                trigger={(open) => <Button variant="danger" onClick={open}>Delete</Button>}
              />
            </div>
          )}
        </>
      )}
    </Drawer>
  );
}
