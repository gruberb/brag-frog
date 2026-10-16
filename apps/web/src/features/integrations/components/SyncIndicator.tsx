import { Link } from 'react-router';
import { paths } from '@/config/paths';
import { relativeTime } from '@/utils/dates';
import { useSyncStatus } from '../api/integrations';

export function SyncIndicator() {
  const { data } = useSyncStatus();
  if (!data) return <div className="sync"><span className="dot gray" /> Sync status…</div>;
  const failed = Object.values(data.services).some((s) => s.state === 'error');
  const last = Object.values(data.services).map((s) => s.last_sync_at).filter(Boolean).sort().at(-1) ?? null;
  return (
    <div className="sync" role="status">
      <span className={`dot ${data.running ? 'amber' : failed ? 'red' : 'green'}`} />
      {data.running ? 'Syncing…' : failed ? 'Sync failed' : last ? `Synced ${relativeTime(last)}` : 'Not synced yet'}
      <Link className="muted" to={paths.settings('integrations')}>
        · details
      </Link>
    </div>
  );
}
