import { Link } from 'react-router';
import { Drawer } from '@/components/ui/Drawer';
import { paths } from '@/config/paths';
import { useConfig } from '@/hooks/use-config';
import { useSession } from '@/stores/session';
import { humanize } from '@/utils/entries';

const Defs = ({ title, items }: { title: string; items: Record<string, string> }) => (
  <div>
    <h3>{title}</h3>
    <dl style={{ margin: '8px 0 0' }}>
      {Object.entries(items).filter(([, v]) => v).map(([k, v]) => (
        <div key={k} style={{ marginBottom: 8 }}>
          <dt className="small muted" style={{ textTransform: 'capitalize' }}>{humanize(k)}</dt>
          <dd style={{ margin: 0 }}>{v}</dd>
        </div>
      ))}
    </dl>
  </div>
);

/** Replaces the old level guide page. */
export function CareerLevelDrawer({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const { user } = useSession();
  const cfg = useConfig();
  const level = cfg.data?.clg_levels.find((l) => l.id === user.role);
  return (
    <Drawer open={open} onOpenChange={onOpenChange} title={level?.title ?? 'Career level'} subtitle={level?.one_liner}>
      {level ? (
        <>
          <Defs title="Problems you solve" items={level.problems} />
          <Defs title="People and influence" items={level.people} />
        </>
      ) : (
        <p>Set your career level in <Link className="link" to={paths.settings('profile')}>Settings</Link> to see its expectations here.</p>
      )}
    </Drawer>
  );
}
