import { useRef, useState } from 'react';
import { useSearchParams } from 'react-router';
import { Button } from '@/components/ui/Button';
import { ChipButton } from '@/components/ui/Chip';
import { Select, type Option } from '@/components/ui/Select';
import type { EntryFacets } from '@/types/api/models';
import { activeFilterCount, parseFilters, writeFilters } from '@/utils/entry-filters';
import { complexityOptions, reachOptions, roleOptions, sourceOptions } from '@/utils/entries';

type Props = { facets?: EntryFacets; goals: Option[]; priorities: Option[] };

const opts = (xs: string[] = []) => xs.map((x) => ({ value: x, label: x }));

/** All filter state lives in the URL, so the Report dialog and shared links see exactly what the list shows. */
export function FilterBar({ facets, goals, priorities }: Props) {
  const [sp, setSp] = useSearchParams();
  const f = parseFilters(sp);
  const [more, setMore] = useState(false);
  // The search box keeps a local draft while typing and commits to the URL after 250 ms.
  const [draft, setDraft] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const patch = (p: Parameters<typeof writeFilters>[1]) => setSp((prev) => writeFilters(prev, p), { replace: true });
  const onSearch = (v: string) => {
    setDraft(v);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      patch({ q: v || undefined });
      setDraft(null);
    }, 250);
  };

  const sel = (key: keyof typeof f, options: Option[], all: string, label: string) => (
    <Select aria-label={label} value={String(f[key] ?? '')} onChange={(e) => patch({ [key]: e.target.value || undefined } as never)} options={options} placeholder={all} />
  );
  const showOnly = f.no_priority ? 'no_priority' : f.no_team ? 'no_team' : f.no_person ? 'no_person' : '';
  const setShowOnly = (v: string) => patch({ no_priority: v === 'no_priority', no_team: v === 'no_team', no_person: v === 'no_person' });
  const filtered = activeFilterCount(f) > 0;

  return (
    <div className="card filter-card">
      <div className="filter-top">
        <input type="search" placeholder="Search entries…" aria-label="Search entries" value={draft ?? f.q ?? ''} onChange={(e) => onSearch(e.target.value)} />
        {sel('goal_id', goals, 'All Dept Goals', 'Department goal')}
        {sel('priority_id', priorities, 'All Priorities', 'Priority')}
        <Button variant="toggle" onClick={() => setMore((m) => !m)} aria-expanded={more}>
          {more ? 'Fewer filters' : 'More filters'}
        </Button>
      </div>
      {more && (
        <div className="filter-more">
          <div className="field"><label>Source</label>{sel('source', sourceOptions, 'All Sources', 'Source')}</div>
          <div className="field"><label>Team</label>{sel('team', opts(facets?.teams), 'All Teams', 'Team')}</div>
          <div className="field"><label>Person</label>{sel('person', opts(facets?.people), 'All People', 'Person')}</div>
          <div className="field"><label>Category</label>{sel('category', (facets?.categories ?? []).map((c) => ({ value: c.value, label: c.label })), 'All Categories', 'Category')}</div>
          <div className="field"><label>Reach</label>{sel('reach', reachOptions, 'All Reach', 'Reach')}</div>
          <div className="field"><label>Complexity</label>{sel('complexity', complexityOptions, 'All Complexity', 'Complexity')}</div>
          <div className="field"><label>Role</label>{sel('role', roleOptions, 'All Roles', 'Role')}</div>
          <div className="field">
            <label>Show only</label>
            <Select aria-label="Show only" value={showOnly} onChange={(e) => setShowOnly(e.target.value)} options={[{ value: 'no_priority', label: 'No priority' }, { value: 'no_team', label: 'No team' }, { value: 'no_person', label: 'No person' }]} placeholder="All entries" />
          </div>
          <div className="field"><label>From</label><input type="date" aria-label="From date" value={f.from ?? ''} onChange={(e) => patch({ from: e.target.value || undefined })} /></div>
          <div className="field"><label>To</label><input type="date" aria-label="To date" value={f.to ?? ''} onChange={(e) => patch({ to: e.target.value || undefined })} /></div>
        </div>
      )}
      <div className="row wrap">
        <ChipButton on={!!f.no_priority} onClick={() => setShowOnly(f.no_priority ? '' : 'no_priority')}>No priority</ChipButton>
        <ChipButton on={!!f.no_team} onClick={() => setShowOnly(f.no_team ? '' : 'no_team')}>No team</ChipButton>
        <ChipButton on={!!f.no_person} onClick={() => setShowOnly(f.no_person ? '' : 'no_person')}>No person</ChipButton>
        {filtered && (
          <button type="button" className="linkish" onClick={() => setSp((prev) => { const n = new URLSearchParams(); const c = prev.get('cycle'); if (c) n.set('cycle', c); return n; }, { replace: true })}>
            Clear filters
          </button>
        )}
      </div>
    </div>
  );
}
