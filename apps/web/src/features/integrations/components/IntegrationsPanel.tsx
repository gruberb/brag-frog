import { useState } from 'react';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { ErrorState } from '@/components/ui/EmptyState';
import { SkeletonRows } from '@/components/ui/Spinner';
import { useConfig } from '@/hooks/use-config';
import { useCycleContext } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { Integration } from '@/types/api/models';
import { relativeTime } from '@/utils/dates';
import { useIntegrations, useSync, useSyncStatus } from '../api/integrations';
import { ConfigureDrawer } from './ConfigureDrawer';

const ICON: Record<string, { abbr: string; cls: string }> = {
  github: { abbr: 'GH', cls: 's-gh' }, phabricator: { abbr: 'Ph', cls: 's-ph' }, bugzilla: { abbr: 'Bz', cls: 's-bz' }, atlassian: { abbr: 'J', cls: 's-jira' },
  google_calendar: { abbr: 'C', cls: 's-cal' }, google_drive: { abbr: 'D', cls: 's-drive' },
};

function subtitle(i: Integration): string {
  if (i.status === 'error') return `last sync failed: ${i.last_error ?? 'unknown error'}`;
  if (i.status === 'not_connected') return '';
  const parts = [i.last_sync_at ? `synced ${relativeTime(i.last_sync_at)}` : 'not synced yet', i.entry_count ? `${i.entry_count} entries` : ''];
  if (i.excluded.length) parts.push(`${i.excluded.length} excluded`);
  return parts.filter(Boolean).join(' · ');
}

export function IntegrationsPanel() {
  const list = useIntegrations();
  const cfg = useConfig();
  const status = useSyncStatus();
  const sync = useSync();
  const { readOnly } = useCycleContext();
  const [openService, setOpenService] = useState<string | null>(null);
  // The AI key has its own tab; it shares the integrations API but is not a sync source.
  const items = list.data?.filter((i) => i.service !== 'claude') ?? [];
  const label = (s: string) => cfg.data?.services.find((x) => x.service === s)?.label ?? s;
  const running = status.data?.running ?? false;

  return (
    <>
      <Card>
        <CardHeader title="Integrations">
          {!readOnly && (
            <Button size="sm" disabled={running || sync.isPending} onClick={() => sync.mutate({}, { onSuccess: () => toast('Sync started'), onError: toastError })}>
              {running ? 'Syncing…' : 'Sync all'}
            </Button>
          )}
        </CardHeader>
        {list.isPending && <CardBody><SkeletonRows rows={4} h={48} /></CardBody>}
        {list.error && <ErrorState error={list.error} onRetry={() => list.refetch()} />}
        {items.map((i) => {
          const icon = ICON[i.service] ?? { abbr: i.service.slice(0, 2), cls: 's-man' };
          return (
            <div key={i.service} className="svc">
              <span className={`src ${icon.cls}`}>{icon.abbr}</span>
              <div><b>{label(i.service)}</b><div className="small muted">{subtitle(i)}</div></div>
              <Badge tone={i.status === 'connected' ? 'green' : i.status === 'error' ? 'red' : 'gray'}>{i.status === 'connected' ? 'Connected' : i.status === 'error' ? 'Error' : 'Not connected'}</Badge>
              <Button size="sm" variant={i.status === 'connected' ? 'default' : 'primary'} onClick={() => setOpenService(i.service)} aria-label={`${i.status === 'connected' ? 'Configure' : i.status === 'error' ? 'Fix token for' : 'Connect'} ${label(i.service)}`}>
                {i.status === 'connected' ? 'Configure' : i.status === 'error' ? 'Fix token' : 'Connect'}
              </Button>
            </div>
          );
        })}
        <CardBody className="small muted">Sync runs in the background. This list shows live status; the activity log is under “Configure”.</CardBody>
      </Card>
      <ConfigureDrawer service={openService} integration={items.find((i) => i.service === openService) ?? null} onClose={() => setOpenService(null)} />
    </>
  );
}
