import { useState, type ReactNode } from 'react';
import { Link } from 'react-router';
import { format, parseISO } from 'date-fns';
import { Badge } from '@/components/ui/Badge';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { Seg } from '@/components/ui/Seg';
import { SkeletonRows } from '@/components/ui/Spinner';
import { paths } from '@/config/paths';
import { useCycleContext, useSession } from '@/stores/session';
import type { EntrySummary, Meeting, PriorityCard } from '@/types/api/models';
import { humanize, sourceMeta, workKind, type WorkKind } from '@/utils/entries';
import { progressPct, trackingMeta } from '@/utils/priorities';
import { useToday } from '../api/today';
import { MeetingPrepDrawer } from './MeetingPrepDrawer';

const greeting = (h: number) => (h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening');
const kindBadge: Record<WorkKind, { label: string; tone: 'blue' | 'purple' | 'gray' }> = {
  code: { label: 'PR', tone: 'blue' },
  review: { label: 'Review', tone: 'purple' },
  ticket: { label: 'Ticket', tone: 'gray' },
  other: { label: 'Other', tone: 'gray' },
};
const TABS: { value: 'all' | WorkKind; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'code', label: 'Code' },
  { value: 'review', label: 'Reviews' },
  { value: 'ticket', label: 'Tickets' },
];

function PriorityRow({ p }: { p: PriorityCard }) {
  const meta = trackingMeta[p.tracking_status ?? 'no_update'];
  const pct = progressPct(p);
  return (
    <div className="prio">
      <div className="row between"><b>{p.title}</b><span className={`badge b-${meta.tone}`}>{meta.label}</span></div>
      {p.due_date && <div className="small muted">due {format(parseISO(p.due_date), 'MMM d')}</div>}
      {pct != null && <div className="bar" style={{ marginTop: 8 }}><i style={{ width: `${pct}%`, background: meta.bar }} /></div>}
      {(pct != null || p.blocker_count > 0) && (
        <div className="small muted" style={{ marginTop: 4 }}>
          {pct != null && `${pct}%`}
          {p.blocker_count > 0 && ` · ${p.blocker_count} blocker${p.blocker_count > 1 ? 's' : ''}${p.latest_update?.is_blocker ? `: ${p.latest_update.comment.slice(0, 40)}` : ''}`}
        </div>
      )}
    </div>
  );
}

function WorkRow({ e }: { e: EntrySummary }) {
  const m = sourceMeta[e.source];
  const k = kindBadge[workKind(e)];
  return (
    <div className="row">
      <span className={`src ${m.cls}`} title={m.label}>{m.abbr}</span>
      <div className="grow">
        {e.source_url ? <a href={e.source_url} target="_blank" rel="noreferrer" className="clamp" style={{ display: 'block' }}>{e.title}</a> : <span className="clamp" style={{ display: 'block' }}>{e.title}</span>}
        <div className="small muted">{[e.repository, humanize(e.status ?? e.entry_type)].filter(Boolean).join(' · ')}</div>
      </div>
      <Badge tone={k.tone}>{k.label}</Badge>
    </div>
  );
}

/** Three blocks only: log, week, priorities and open work. `composer` comes from features/entries via the route. */
export function TodayPage({ composer }: { composer: ReactNode }) {
  const { cycle } = useCycleContext();
  const { user } = useSession();
  const today = useToday(cycle?.id ?? null);
  const [tab, setTab] = useState<'all' | WorkKind>('all');
  const [prepFor, setPrepFor] = useState<Meeting | null>(null);
  const d = today.data;
  const name = user.display_name ?? user.name.split(' ')[0];

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{greeting(new Date().getHours())}, {name}</h1>
          <p>{d ? `Week ${d.week.iso_week} · ${format(parseISO(d.week.week_start), 'MMM d')} to ${format(parseISO(d.week.week_end), 'd')}` : 'This week'}{cycle ? ` · ${cycle.name}` : ''}</p>
        </div>
      </div>

      <Card style={{ marginBottom: 16 }}><CardBody>{composer}</CardBody></Card>

      {today.isPending && <SkeletonRows rows={4} h={90} />}
      {today.error && <ErrorState error={today.error} onRetry={() => today.refetch()} />}
      {d && (
        <>
          <Card style={{ marginBottom: 16 }}>
            <CardHeader title="This week"><span className="muted small">{d.counts.meetings} meetings · {d.counts.focus_hours}h focus time</span></CardHeader>
            <CardBody>
              <div className="week">
                {d.meetings_by_day.map((day) => (
                  <div key={day.date} className={`day ${day.is_today ? 'today' : ''}`}>
                    <h4>{day.label}</h4>
                    {day.meetings.map((m) => {
                      const prepped = d.prep_map[String(m.entry_id)];
                      return (
                        <button key={m.entry_id} type="button" className={`mtg ${prepped ? 'prep' : ''}`} onClick={() => setPrepFor(m)}>
                          <time>{m.start_time}</time>
                          {m.title}
                          {prepped && <> · <b>prep ready</b></>}
                        </button>
                      );
                    })}
                    {day.meetings.length === 0 && <span className="small muted">No meetings</span>}
                  </div>
                ))}
              </div>
            </CardBody>
          </Card>

          <div className="layout">
            <Card>
              <CardHeader title="Priorities"><Link className="muted small" to={paths.priorities}>All priorities →</Link></CardHeader>
              {d.priorities.length === 0 ? <EmptyState title="No active priorities" /> : d.priorities.slice(0, 4).map((p) => <PriorityRow key={p.id} p={p} />)}
            </Card>
            <Card>
              <CardHeader title="Open work"><Seg label="Open work filter" options={TABS} value={tab} onChange={setTab} /></CardHeader>
              <div className="list">
                {d.active_work.filter((e) => tab === 'all' || workKind(e) === tab).map((e) => <WorkRow key={e.id} e={e} />)}
                {d.active_work.filter((e) => tab === 'all' || workKind(e) === tab).length === 0 && <EmptyState title="Nothing open here" />}
              </div>
            </Card>
          </div>
        </>
      )}
      <MeetingPrepDrawer meeting={prepFor} onClose={() => setPrepFor(null)} />
    </>
  );
}
