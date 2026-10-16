import { useState } from 'react';
import { Link } from 'react-router';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { ConfirmAction } from '@/components/ui/ConfirmAction';
import { ErrorState } from '@/components/ui/EmptyState';
import { Menu } from '@/components/ui/Menu';
import { SkeletonRows } from '@/components/ui/Spinner';
import { paths } from '@/config/paths';
import { useJiraBase } from '@/hooks/use-config';
import type { ExportFormat } from '@/lib/export';
import { shortDate } from '@/utils/dates';
import { toast, toastError } from '@/stores/toast';
import type { CycleSummary } from '@/types/api/models';
import { fetchCycleBundle, useActivateCycle, useCycles, useDeleteCycle } from '../api/cycles';
import { CycleDialog } from './CycleDialog';

type Kind = ExportFormat | 'json';

export function CyclesPanel() {
  const q = useCycles();
  const jiraBase = useJiraBase();
  const activate = useActivateCycle();
  const del = useDeleteCycle();
  const [creating, setCreating] = useState(false);
  const [busy, setBusy] = useState<number | null>(null);

  const download = async (c: CycleSummary, kind: Kind) => {
    setBusy(c.id);
    try {
      const bundle = await fetchCycleBundle(c.id);
      // PDF/DOCX libraries are loaded only when a download is requested.
      const { downloadCycleBundle } = await import('@/lib/export');
      await downloadCycleBundle(kind, bundle, jiraBase);
      toast(`${c.name} downloaded`);
    } catch (e) {
      toastError(e);
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      <Card>
        <CardHeader title="Performance cycles"><Button variant="primary" size="sm" onClick={() => setCreating(true)}>+ New cycle</Button></CardHeader>
        {q.isPending && <CardBody><SkeletonRows rows={3} h={52} /></CardBody>}
        {q.error && <ErrorState error={q.error} onRetry={() => q.refetch()} />}
        <div className="list">
          {q.data?.map((c) => (
            <div key={c.id} className="row between wrap">
              <div>
                <b>{c.name}</b> <Badge tone={c.is_active ? 'green' : 'gray'}>{c.is_active ? 'Active' : 'Past'}</Badge>
                <div className="small muted">{shortDate(c.start_date)} to {shortDate(c.end_date)} · {c.entry_count} entries · {c.priority_count} priorities</div>
              </div>
              <div className="row wrap">
                {!c.is_active && <Button size="sm" variant="action" disabled={activate.isPending} onClick={() => activate.mutate(c.id, { onSuccess: () => toast(`${c.name} is now active`), onError: toastError })}>Make active</Button>}
                <Link className="btn sm" to={c.is_active ? paths.review(c.id) : { pathname: paths.review(c.id), search: `?cycle=${c.id}` }}>{c.is_active ? 'Review' : 'View'}</Link>
                <Menu
                  trigger={<Button size="sm" variant="transfer" icon={busy === c.id ? undefined : '⬇'} disabled={busy === c.id} aria-label={`Download ${c.name}`}>{busy === c.id ? 'Preparing…' : 'Download ⌄'}</Button>}
                  items={[
                    { label: 'Markdown', onSelect: () => void download(c, 'markdown') },
                    { label: 'PDF', onSelect: () => void download(c, 'pdf') },
                    { label: 'DOCX', onSelect: () => void download(c, 'docx') },
                    { label: 'Raw JSON (backup)', onSelect: () => void download(c, 'json') },
                  ]}
                />
                {!c.is_active && (
                  <ConfirmAction
                    title={`Delete ${c.name}`}
                    requireText={c.name}
                    message="This removes the cycle and all of its entries, priorities, updates and self-review. It cannot be undone."
                    onConfirm={() => del.mutateAsync({ id: c.id, confirm_name: c.name }).then(() => toast('Cycle deleted')).catch(toastError)}
                    trigger={(open) => <Button size="sm" variant="danger" onClick={open}>Delete</Button>}
                  />
                )}
              </div>
            </div>
          ))}
        </div>
        <CardBody className="small muted">Download bundle: self-review (written), priorities with updates, entries grouped by week. Formats: Markdown, PDF, DOCX. Deleting a cycle removes all of its data and asks you to type its name.</CardBody>
      </Card>
      <CycleDialog open={creating} onOpenChange={setCreating} />
    </>
  );
}
