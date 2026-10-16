import { useState } from 'react';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { ErrorState } from '@/components/ui/EmptyState';
import { SkeletonRows } from '@/components/ui/Spinner';
import { useCycleContext } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { Integration } from '@/types/api/models';
import { relativeTime } from '@/utils/dates';
import { useIntegrations, useSync, useSyncStatus } from '../api/integrations';
import { integrationState } from '../utils/integration-status';
import { ConfigureDrawer } from './ConfigureDrawer';

const ICON: Record<string, { abbr: string; cls: string }> = {
  github: { abbr: 'GH', cls: 's-gh' }, phabricator: { abbr: 'Ph', cls: 's-ph' }, bugzilla: { abbr: 'Bz', cls: 's-bz' }, atlassian: { abbr: 'J', cls: 's-jira' },
  google_calendar: { abbr: 'C', cls: 's-cal' }, google_drive: { abbr: 'D', cls: 's-drive' },
};

function subtitle(i: Integration): string {
  const state = integrationState(i);
  if (state === 'error') return `last sync failed: ${i.last_sync_error ?? 'unknown error'}`;
  if (state === 'not_connected') return '';
  const excluded = i.excluded_files.length + i.excluded_events.length;
  const parts = [i.last_sync_at ? `synced ${relativeTime(i.last_sync_at)}` : 'not synced yet', i.entry_count ? `${i.entry_count} entries` : ''];
  if (excluded) parts.push(`${excluded} excluded`);
  return parts.filter(Boolean).join(' · ');
}

export function IntegrationsPanel() {
  const list = useIntegrations();
  const status = useSyncStatus();
  const sync = useSync();
  const { readOnly } = useCycleContext();
  const [openService, setOpenService] = useState<string | null>(null);
  // The AI key has its own tab; it shares the integrations API but is not a sync source.
  const items = list.data?.filter((i) => !i.token_only) ?? [];
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
          const state = integrationState(i);
          return (
            <div key={i.service} className="svc">
              <span className={`src ${icon.cls}`}>{icon.abbr}</span>
              <div><b>{i.display_name}</b><div className="small muted">{subtitle(i)}</div></div>
              <Badge tone={state === 'connected' ? 'green' : state === 'error' ? 'red' : 'gray'}>{state === 'connected' ? 'Connected' : state === 'error' ? 'Error' : 'Not connected'}</Badge>
              <Button size="sm" variant={state === 'connected' ? 'default' : 'primary'} onClick={() => (state === 'not_connected' && i.oauth_connect ? window.location.assign(`/auth/connect/${i.service}`) : setOpenService(i.service))} aria-label={`${state === 'connected' ? 'Configure' : state === 'error' ? 'Fix token for' : 'Connect'} ${i.display_name}`}>
                {state === 'connected' ? 'Configure' : state === 'error' ? 'Fix token' : 'Connect'}
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
