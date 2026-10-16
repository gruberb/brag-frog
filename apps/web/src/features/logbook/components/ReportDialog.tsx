import { useMemo, useState } from 'react';
import { useSearchParams } from 'react-router';
import { Button } from '@/components/ui/Button';
import { Chip } from '@/components/ui/Chip';
import { ConfirmAction } from '@/components/ui/ConfirmAction';
import { Dialog } from '@/components/ui/Dialog';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { Seg } from '@/components/ui/Seg';
import { SkeletonRows } from '@/components/ui/Spinner';
import { TabList } from '@/components/ui/Tabs';
import { useJiraBase } from '@/hooks/use-config';
import { ApiError } from '@/lib/api-client';
import { useCycleContext, useSession } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { Report } from '@/types/api/models';
import { parseFilters } from '@/utils/entry-filters';
import { describeFilters, hasListOnlyFilters, toReportFilters } from '../utils/describe-filters';
import { shortDate } from '@/utils/dates';
import { fetchReport, useCreateReport, useDeleteReport, useReports } from '../api/reports';
import type { ExportFormat } from '@/lib/export';

type Props = {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  /** Number of entries matching the current filters, from the list query. */
  matching: number;
  priorityNames: Record<string, string>;
};

const FORMATS: { value: ExportFormat; label: string }[] = [
  { value: 'pdf', label: 'PDF' },
  { value: 'docx', label: 'DOCX' },
  { value: 'markdown', label: 'Markdown' },
];
const SHORT: Record<ExportFormat, string> = { pdf: 'PDF', docx: 'DOCX', markdown: 'MD' };

async function download(format: ExportFormat, report: Report, jiraBase: string | null | undefined) {
  // Heavy PDF/DOCX libraries load only when a download is requested.
  const { downloadReport } = await import('@/lib/export');
  await downloadReport(format, report, jiraBase);
}

export function ReportDialog({ open, onOpenChange, matching, priorityNames }: Props) {
  const [tab, setTab] = useState('new');
  const { cycle, readOnly } = useCycleContext();
  const { has_ai } = useSession();
  const jiraBase = useJiraBase();
  const [sp] = useSearchParams();
  const filters = useMemo(() => parseFilters(sp), [sp]);
  const chips = describeFilters(filters, priorityNames);
  const defaultTitle = `${filters.priority_id ? (priorityNames[String(filters.priority_id)] ?? 'Project') : 'All projects'} · ${chips.find((c) => /to|From|Until/.test(c)) ?? cycle?.name ?? 'report'}`;
  const [title, setTitle] = useState('');
  const [withAi, setWithAi] = useState(true);
  const [format, setFormat] = useState<ExportFormat>('pdf');
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const create = useCreateReport();
  const reports = useReports(cycle?.id ?? null, open && tab === 'past');
  const del = useDeleteReport();
  const [busyId, setBusyId] = useState<number | null>(null);

  const generate = async () => {
    if (!cycle) return;
    setError(null);
    setNote(null);
    try {
      const report = await create.mutateAsync({ cycle_id: cycle.id, title: title.trim() || defaultTitle, filters: toReportFilters(filters), with_ai: has_ai && withAi });
      await download(format, report, jiraBase);
      if (report.ai_error) setNote(`Report saved without an AI summary: ${report.ai_error}`);
      else toast('Report saved and downloaded');
      if (!report.ai_error) onOpenChange(false);
    } catch (e) {
      if (e instanceof ApiError && e.code === 'empty_selection') setError('No entries match the current filters. Widen them and try again.');
      else setError(e instanceof Error ? e.message : 'Could not create the report');
    }
  };

  const redownload = async (id: number, f: ExportFormat) => {
    setBusyId(id);
    try {
      await download(f, await fetchReport(id), jiraBase);
    } catch (e) {
      toastError(e);
    } finally {
      setBusyId(null);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="Create report">
      <div style={{ padding: '0 16px' }}>
        <TabList value={tab} onChange={setTab} items={[{ value: 'new', label: 'New' }, { value: 'past', label: `Past reports${reports.data ? ` (${reports.data.length})` : ''}` }]} />
      </div>
      {tab === 'new' ? (
        <div className="card-b col" style={{ gap: 16 }}>
          <div>
            <div className="small muted">Using the current Logbook filters</div>
            <div className="row wrap" style={{ marginTop: 6 }}>
              {chips.length === 0 && <Chip on>Whole cycle</Chip>}
              {chips.map((c) => <Chip key={c} on>{c}</Chip>)}
              <b style={{ marginLeft: 'auto' }}>{matching} {matching === 1 ? 'entry' : 'entries'}</b>
            </div>
          </div>
          <div className="field">
            <label htmlFor="report-title">Title</label>
            <input id="report-title" value={title} placeholder={defaultTitle} onChange={(e) => setTitle(e.target.value)} />
          </div>
          {has_ai && (
            <label className="row small" style={{ gap: 8 }}>
              <input type="checkbox" checked={withAi} onChange={(e) => setWithAi(e.target.checked)} /> Include an AI summary
            </label>
          )}
          <div className="field">
            <label>Format</label>
            <Seg label="Format" options={FORMATS} value={format} onChange={setFormat} />
          </div>
          {hasListOnlyFilters(filters) && <div className="small muted" role="note">The category and “no …” views are not applied to reports, so the report may contain more entries than the list.</div>}
          <div className="small muted">Contains: summary, entries per week, who you worked with, projects, key entries. Saved to Past reports.</div>
          {error && <div className="err-text" role="alert">{error}</div>}
          {note && <div className="small" role="status">{note}</div>}
          <div className="row between">
            <span className="small muted">{create.isPending ? (withAi && has_ai ? 'Generating, the AI summary can take up to two minutes…' : 'Generating…') : ''}</span>
            <Button variant="primary" onClick={generate} disabled={create.isPending || matching === 0 || !cycle}>
              {create.isPending ? 'Generating…' : readOnly ? 'Generate and download' : 'Generate and download'}
            </Button>
          </div>
        </div>
      ) : (
        <div>
          {reports.isPending && <div className="card-b"><SkeletonRows rows={3} /></div>}
          {reports.error && <ErrorState error={reports.error} onRetry={() => reports.refetch()} />}
          {reports.data?.length === 0 && <EmptyState title="No reports yet">Create one from the New tab.</EmptyState>}
          <div className="list">
            {reports.data?.map((r) => (
              <div key={r.id} className="row between wrap">
                <div className="grow">
                  <b style={{ fontWeight: 500 }}>{r.title}</b>
                  <div className="small muted">{r.entry_count} entries · {shortDate(r.generated_at.slice(0, 10))}</div>
                </div>
                <div className="row">
                  {FORMATS.map((f) => (
                    <Button key={f.value} size="sm" disabled={busyId === r.id} onClick={() => redownload(r.id, f.value)} aria-label={`Download ${r.title} as ${f.label}`}>
                      {SHORT[f.value]}
                    </Button>
                  ))}
                  <ConfirmAction
                    title="Delete report"
                    message={`Delete “${r.title}”? This cannot be undone.`}
                    onConfirm={() => del.mutateAsync(r.id).catch(toastError)}
                    trigger={(openConfirm) => <Button size="sm" variant="ghost" onClick={openConfirm} aria-label={`Delete ${r.title}`}>Delete</Button>}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </Dialog>
  );
}
