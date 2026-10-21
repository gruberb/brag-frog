import { useState } from 'react';
import { Link } from 'react-router';
import { Drawer } from '@/components/ui/Drawer';
import { paths } from '@/config/paths';
import { useConfig } from '@/hooks/use-config';
import { useSession } from '@/stores/session';
import type { ClgLevel } from '@/types/api/models';
import { humanize } from '@/utils/entries';

type Group = 'problems' | 'people';

/** One expectation group. With `next`, each dimension also shows the next level's text, so the
 * difference is visible line by line while writing. */
const Defs = ({ title, group, level, next }: { title: string; group: Group; level: ClgLevel; next?: ClgLevel }) => {
  const current: Record<string, string> = level[group];
  const upcoming: Record<string, string> | undefined = next?.[group];
  const keys = Object.keys(current).filter((k) => current[k] || upcoming?.[k]);
  return (
    <div>
      <h3>{title}</h3>
      <dl style={{ margin: '8px 0 0' }}>
        {keys.map((k) => (
          <div key={k} style={{ marginBottom: 8 }}>
            <dt className="small muted" style={{ textTransform: 'capitalize' }}>{humanize(k)}</dt>
            <dd style={{ margin: 0 }}>{current[k] || <span className="muted">Not defined</span>}</dd>
            {next && upcoming?.[k] && (
              <dd style={{ margin: '4px 0 0' }}><span className="small muted">Next: </span>{upcoming[k]}</dd>
            )}
          </div>
        ))}
      </dl>
    </div>
  );
};

/** Replaces the old level guide page. */
export function CareerLevelDrawer({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const { user } = useSession();
  const cfg = useConfig();
  const levels = cfg.data?.clg_levels ?? [];
  const idx = levels.findIndex((l) => l.id === user.role);
  const level = idx >= 0 ? levels[idx] : undefined;
  // Levels arrive in ladder order (the same order the server's `get_next_level` uses).
  const nextLevel = idx >= 0 ? levels[idx + 1] : undefined;
  const [compare, setCompare] = useState(user.wants_promotion);
  const next = compare ? nextLevel : undefined;
  return (
    <Drawer open={open} onOpenChange={onOpenChange} title={level?.title ?? 'Career level'} subtitle={level?.one_liner}>
      {level ? (
        <>
          {nextLevel && (
            <label className="small row">
              <input type="checkbox" checked={compare} onChange={(e) => setCompare(e.target.checked)} /> Compare with {nextLevel.title}
            </label>
          )}
          {next && <p className="small"><span className="muted">Next level: </span>{next.one_liner}</p>}
          <Defs title="Problems you solve" group="problems" level={level} next={next} />
          <Defs title="People and influence" group="people" level={level} next={next} />
        </>
      ) : (
        <p>Set your career level in <Link className="link" to={paths.settings('profile')}>Settings</Link> to see its expectations here.</p>
      )}
    </Drawer>
  );
}
