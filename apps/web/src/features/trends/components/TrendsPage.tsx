import { useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router';
import { format, parseISO } from 'date-fns';
import { Button } from '@/components/ui/Button';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { ErrorState } from '@/components/ui/EmptyState';
import { Seg } from '@/components/ui/Seg';
import { Skeleton } from '@/components/ui/Spinner';
import { paths } from '@/config/paths';
import { downloadFile } from '@/lib/download';
import { useCycleContext } from '@/stores/session';
import type { Trends, TrendRange } from '@/types/api/models';
import { useTrends } from '../api/trends';
import { RankList } from './RankList';
import { weekRange, weekTotal } from '../utils/weeks';
import { StackedBars } from './StackedBars';

const RANGES: { value: TrendRange; label: string }[] = [
  { value: '4w', label: '4 weeks' },
  { value: 'quarter', label: 'Quarter' },
  { value: 'cycle', label: 'Cycle' },
];

function ShowData({ children, summary }: { children: React.ReactNode; summary: string }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <p className="sr-only">{summary}</p>
      <button type="button" className="linkish small" aria-expanded={open} onClick={() => setOpen((o) => !o)} style={{ marginTop: 8 }}>
        {open ? 'Hide data' : 'Show data'}
      </button>
      {open && <div style={{ marginTop: 8, overflowX: 'auto' }}>{children}</div>}
    </>
  );
}

const Kpi = ({ n, l }: { n: string | number; l: string }) => (
  <Card className="kpi"><div className="n">{n}</div><div className="l">{l}</div></Card>
);

function toCsv(t: Trends) {
  const rows = [['week_start', 'code', 'review', 'meeting', 'other', 'total'], ...t.weekly.map((w) => [w.week_start, w.by_type.code, w.by_type.review, w.by_type.meeting, w.by_type.other, weekTotal(w)])];
  return rows.map((r) => r.join(',')).join('\n');
}

export function TrendsPage() {
  const { cycle } = useCycleContext();
  const nav = useNavigate();
  const [sp, setSp] = useSearchParams();
  const range = (RANGES.find((r) => r.value === sp.get('range'))?.value ?? 'cycle') as TrendRange;
  const q = useTrends(cycle?.id ?? null, range);
  const t = q.data;
  const prioritised = t ? t.priority_share.reduce((n, p) => n + p.count, 0) : 0;
  const linkedPct = t && prioritised > 0 ? Math.round((t.priority_share.filter((p) => p.priority_id != null).reduce((n, p) => n + p.count, 0) / prioritised) * 100) : 0;
  const cycleSearch = cycle && !cycle.is_active ? `&cycle=${cycle.id}` : '';
  const logbook = (params: string) => `${paths.logbook}?${params}${cycleSearch}`;

  return (
    <>
      <div className="page-head">
        <div><h1>Trends</h1><p>Where your time went, with whom, and on what.</p></div>
        <div className="row">
          <Seg label="Range" options={RANGES} value={range} onChange={(r) => setSp((prev) => { const n = new URLSearchParams(prev); n.set('range', r); return n; }, { replace: true })} />
          <Button variant="transfer" icon="⬇" disabled={!t} onClick={() => t && downloadFile(`trends-${range}.csv`, toCsv(t), 'text/csv')}>Export CSV</Button>
        </div>
      </div>

      {q.isPending && <div className="grid g4"><Skeleton h={76} /><Skeleton h={76} /><Skeleton h={76} /><Skeleton h={76} /></div>}
      {q.error && <ErrorState error={q.error} onRetry={() => q.refetch()} />}
      {t && (
        <div style={{ opacity: q.isPlaceholderData ? 0.6 : 1 }}>
          <div className="grid g4" style={{ marginBottom: 16 }}>
            <Kpi n={t.kpis.total} l="entries" />
            <Kpi n={t.kpis.prs_merged} l="PRs merged" />
            <Kpi n={t.kpis.prs_reviewed + t.kpis.jira_closed} l="reviews and tickets closed" />
            <Kpi n={`${linkedPct}%`} l="linked to a priority" />
          </div>

          <div className="grid" style={{ gridTemplateColumns: 'minmax(0, 2fr) minmax(0, 1fr)', marginBottom: 16, alignItems: 'start' }}>
            <Card>
              <CardHeader title="Activity by week"><span className="small muted">stacked by type</span></CardHeader>
              <CardBody>
                <StackedBars weeks={t.weekly} onSelectWeek={(w) => void nav(logbook(`from=${weekRange(w).from}&to=${weekRange(w).to}`))} />
                <ShowData summary={`${t.weekly.length} weeks, ${t.weekly.reduce((n, w) => n + weekTotal(w), 0)} entries in total. Busiest week starts ${t.weekly.slice().sort((a, b) => weekTotal(b) - weekTotal(a))[0] ? format(parseISO(t.weekly.slice().sort((a, b) => weekTotal(b) - weekTotal(a))[0].week_start), 'MMM d') : 'n/a'}.`}>
                  <table className="small" style={{ borderCollapse: 'collapse', width: '100%' }}>
                    <thead><tr>{['Week', 'Code', 'Review', 'Meetings', 'Other', 'Total'].map((h) => <th key={h} style={{ textAlign: 'left', padding: '2px 8px', fontWeight: 500 }}>{h}</th>)}</tr></thead>
                    <tbody>
                      {t.weekly.map((w) => (
                        <tr key={w.week_start}>
                          <td style={{ padding: '2px 8px' }}>{w.week_start}</td><td style={{ padding: '2px 8px' }}>{w.by_type.code}</td><td style={{ padding: '2px 8px' }}>{w.by_type.review}</td>
                          <td style={{ padding: '2px 8px' }}>{w.by_type.meeting}</td><td style={{ padding: '2px 8px' }}>{w.by_type.other}</td><td style={{ padding: '2px 8px' }}>{weekTotal(w)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </ShowData>
              </CardBody>
            </Card>
            <Card>
              <CardHeader title="Priority share" />
              <CardBody className="col">
                {(() => {
                  const total = Math.max(1, t.priority_share.reduce((n, p) => n + p.count, 0));
                  const palette = ['var(--color-primary)', 'var(--color-orange-40)', 'var(--color-purple-30)', 'var(--color-blue-30)', 'var(--color-success)'];
                  return t.priority_share.map((p, i) => {
                    const pct = Math.round((p.count / total) * 100);
                    return (
                      <div key={p.priority_id ?? 'none'}>
                        <div className="row between small"><span>{p.title}</span><b>{pct}%</b></div>
                        <div className="bar" role="img" aria-label={`${p.title}: ${pct}%`}><i style={{ width: `${pct}%`, background: p.priority_id == null ? 'var(--color-gray-40)' : palette[i % palette.length] }} /></div>
                      </div>
                    );
                  });
                })()}
                <ShowData summary={`Share of entries by priority: ${t.priority_share.map((p) => `${p.title} ${p.count}`).join(', ')}.`}>
                  <table className="small"><tbody>{t.priority_share.map((p) => <tr key={p.priority_id ?? 'none'}><td style={{ padding: '2px 8px' }}>{p.title}</td><td>{p.count}</td></tr>)}</tbody></table>
                </ShowData>
              </CardBody>
            </Card>
          </div>

          <div className="grid g4" style={{ marginBottom: 16 }}>
            <RankList title="Categories" items={t.categories} color="var(--color-primary)" to={(n) => logbook(`category=${encodeURIComponent(n.toLowerCase())}`)} />
            <RankList title="People" items={t.people} color="var(--color-blue-30)" to={(n) => logbook(`person=${encodeURIComponent(n)}`)} />
            <RankList title="Repositories" items={t.repos} color="var(--color-success)" mono to={(n) => logbook(`q=${encodeURIComponent(n)}`)} />
            <RankList title="Teams" items={t.teams} color="var(--color-orange-40)" to={(n) => logbook(`team=${encodeURIComponent(n)}`)} />
          </div>

          <Card>
            <CardHeader title="When you log"><span className="small muted">logged at, by weekday</span></CardHeader>
            <CardBody>
              {(() => {
                const max = Math.max(1, ...t.weekday.map((d) => d.count));
                return (
                  <div className="row" style={{ gap: 6 }} role="img" aria-label={`Entries by weekday: ${t.weekday.map((d) => `${d.name} ${d.count}`).join(', ')}`}>
                    {t.weekday.map((d) => (
                      <div key={d.name} className="grow" style={{ textAlign: 'center' }}>
                        <div style={{ height: 22, borderRadius: 4, background: 'var(--color-primary)', opacity: 0.12 + 0.7 * (d.count / max) }} title={`${d.name}: ${d.count}`} />
                        <span className="small muted">{d.name.slice(0, 3)}</span>
                      </div>
                    ))}
                  </div>
                );
              })()}
            </CardBody>
          </Card>
        </div>
      )}
    </>
  );
}
