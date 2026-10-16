import { Link } from 'react-router';
import { Card, CardHeader } from '@/components/ui/Card';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { SkeletonRows } from '@/components/ui/Spinner';
import { paths } from '@/config/paths';
import { useArchive } from '../api/review';

type Item = { id: number; title: string; sub?: string; body: string[] };

const Block = ({ title, items }: { title: string; items: Item[] }) => (
  <Card style={{ marginBottom: 16 }}>
    <CardHeader title={title} />
    {items.length === 0 ? <EmptyState title="Nothing archived" /> : (
      <div className="list">
        {items.map((i) => (
          <div key={i.id}>
            <b style={{ fontWeight: 500 }}>{i.title}</b>{i.sub && <span className="small muted"> · {i.sub}</span>}
            {i.body.filter(Boolean).map((t, n) => <div key={n} style={{ whiteSpace: 'pre-wrap', marginTop: 4 }}>{t}</div>)}
          </div>
        ))}
      </div>
    )}
  </Card>
);

/** Read-only view of features removed in the redesign. The data stays in the database. */
export function ArchivePage({ cycleId }: { cycleId: number }) {
  const q = useArchive(cycleId);
  const a = q.data;
  return (
    <>
      <div className="page-head">
        <div><h1>Archive</h1><p>Read-only. <Link className="link" to={paths.review(cycleId)}>← Back to the review</Link></p></div>
      </div>
      {q.isPending && <SkeletonRows rows={3} h={80} />}
      {q.error && <ErrorState error={q.error} onRetry={() => q.refetch()} />}
      {a && (
        <>
          <Block
            title="Check-ins"
            items={a.checkins.map((c) => ({
              id: c.id, title: `${c.quarter} ${c.year}`, sub: c.updated_at.slice(0, 10),
              body: [c.highlights_impact, c.learnings_adjustments, c.growth_development, c.support_feedback, c.looking_ahead].map((t) => t ?? ''),
            }))}
          />
          <Block
            title="Contribution examples"
            items={a.contribution_examples.map((c) => ({
              id: c.id, title: c.title, sub: [c.status, c.impact_level].filter(Boolean).join(' · '),
              body: [c.outcome ?? '', c.behaviors ?? '', c.learnings ?? '', c.entries.length ? `Linked: ${c.entries.map((e) => e.title).join(', ')}` : ''],
            }))}
          />
          <Block title="Status updates" items={a.status_updates.map((c) => ({ id: c.id, title: c.week_start ? `Week of ${c.week_start}` : 'Status update', sub: c.updated_at.slice(0, 10), body: [c.content ?? ''] }))} />
          <Block title="Last-week reports" items={a.last_week_reports.map((c) => ({ id: c.id, title: `${c.window_start} to ${c.window_end}`, sub: c.updated_at.slice(0, 10), body: [c.content ?? ''] }))} />
        </>
      )}
    </>
  );
}
