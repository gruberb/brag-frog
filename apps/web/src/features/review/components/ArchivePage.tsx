import { Link } from 'react-router';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { SkeletonRows } from '@/components/ui/Spinner';
import { paths } from '@/config/paths';
import { useArchive } from '../api/review';

const Block = ({ title, items }: { title: string; items: { id: number; title: string; content: string; sub?: string }[] }) => (
  <Card style={{ marginBottom: 16 }}>
    <CardHeader title={title} />
    {items.length === 0 ? <EmptyState title="Nothing archived" /> : (
      <div className="list">
        {items.map((i) => (
          <div key={i.id}>
            <b style={{ fontWeight: 500 }}>{i.title}</b>{i.sub && <span className="small muted"> · {i.sub}</span>}
            <div style={{ whiteSpace: 'pre-wrap', marginTop: 4 }}>{i.content}</div>
          </div>
        ))}
      </div>
    )}
  </Card>
);

/** Read-only view of features removed in the redesign. The data stays in the database. */
export function ArchivePage({ cycleId }: { cycleId: number }) {
  const q = useArchive(cycleId);
  return (
    <>
      <div className="page-head">
        <div><h1>Archive</h1><p>Read-only. <Link className="link" to={paths.review(cycleId)}>← Back to the review</Link></p></div>
      </div>
      {q.isPending && <SkeletonRows rows={3} h={80} />}
      {q.error && <ErrorState error={q.error} onRetry={() => q.refetch()} />}
      {q.data && (
        <>
          <Block title="Check-ins" items={q.data.checkins.map((c) => ({ id: c.id, title: c.title, content: c.content, sub: c.period }))} />
          <Block title="Contribution examples" items={q.data.contribution_examples.map((c) => ({ id: c.id, title: c.title, content: c.content, sub: c.entry_titles.join(', ') }))} />
          <Block title="Status updates" items={q.data.status_updates.map((c) => ({ id: c.id, title: c.title, content: c.content, sub: c.created_at.slice(0, 10) }))} />
          <Block title="Last-week reports" items={q.data.last_week_reports.map((c) => ({ id: c.id, title: c.title, content: c.content, sub: c.created_at.slice(0, 10) }))} />
          <CardBody />
        </>
      )}
    </>
  );
}
