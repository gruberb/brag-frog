import { Link } from 'react-router';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';

type Props = { title: string; items: { name: string; count: number }[]; color: string; to?: (name: string) => string; mono?: boolean };

/** Top-N bars. When `to` is given, each row links to the logbook filtered by that value. */
export function RankList({ title, items, color, to, mono }: Props) {
  const max = Math.max(1, ...items.map((i) => i.count));
  return (
    <Card>
      <CardHeader title={title} />
      <CardBody className="rank">
        {items.length === 0 && <span className="small muted">Nothing yet</span>}
        {items.map((i) => {
          const row = (
            <>
              <span className={mono ? 'mono clamp' : 'clamp'}>{i.name}</span>
              <div className="bar"><i style={{ width: `${(i.count / max) * 100}%`, background: color }} /></div>
              <b>{i.count}</b>
            </>
          );
          return to ? (
            <Link key={i.name} to={to(i.name)} className="rank-row" style={{ display: 'grid', gridTemplateColumns: '110px 1fr 36px', gap: 8, alignItems: 'center', marginBottom: 8, fontSize: 13 }} aria-label={`${i.name}: ${i.count}. Open in logbook`}>
              {row}
            </Link>
          ) : <div key={i.name}>{row}</div>;
        })}
      </CardBody>
    </Card>
  );
}
