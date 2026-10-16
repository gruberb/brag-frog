import { Link } from 'react-router';
import { paths } from '@/config/paths';
import { relativeTime } from '@/utils/dates';
import { useSyncStatus } from '../api/integrations';

export function SyncIndicator() {
  const { data } = useSyncStatus();
  if (!data) return <div className="sync"><span className="dot gray" /> Sync status…</div>;
  const failed = !!data.last_error;
  return (
    <div className="sync" role="status">
      <span className={`dot ${data.running ? 'amber' : failed ? 'red' : 'green'}`} />
      {data.running ? 'Syncing…' : failed ? 'Sync failed' : data.last_synced_at ? `Synced ${relativeTime(data.last_synced_at)}` : 'Not synced yet'}
      <Link className="muted" to={paths.settings('integrations')}>
        · details
      </Link>
    </div>
  );
}
