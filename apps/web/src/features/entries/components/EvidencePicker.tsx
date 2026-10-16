import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Drawer } from '@/components/ui/Drawer';
import { Select } from '@/components/ui/Select';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { SkeletonRows } from '@/components/ui/Spinner';
import { useDebouncedValue } from '@/hooks/use-debounced-value';
import { usePriorityOptions } from '@/hooks/use-priority-options';
import { useCycleContext } from '@/stores/session';
import type { EntrySummary } from '@/types/api/models';
import { shortDate } from '@/utils/dates';
import { useEntries } from '../api/entries';
import { SourceIcon } from './SourceIcon';

type Props = { open: boolean; onOpenChange: (o: boolean) => void; onInsert: (entry: EntrySummary) => void };

/** Right-hand drawer used by Review: search the logbook and insert an `entryRef` at the cursor. */
export function EvidencePicker({ open, onOpenChange, onInsert }: Props) {
  const { cycle } = useCycleContext();
  const { priorities } = usePriorityOptions(cycle?.id);
  const [q, setQ] = useState('');
  const [priority, setPriority] = useState('');
  const dq = useDebouncedValue(q, 250);
  const entries = useEntries(cycle?.id ?? null, { q: dq || undefined, priority_id: priority ? Number(priority) : undefined }, open);
  const items = entries.data?.pages.flatMap((p) => p.items) ?? [];
  return (
    <Drawer open={open} onOpenChange={onOpenChange} title="Add evidence">
      <input placeholder="Search your entries…" aria-label="Search entries" value={q} onChange={(e) => setQ(e.target.value)} />
      <Select aria-label="Project" value={priority} onChange={(e) => setPriority(e.target.value)} options={priorities} placeholder="All projects" />
      {entries.isPending && <SkeletonRows rows={5} h={44} />}
      {entries.error && <ErrorState error={entries.error} onRetry={() => entries.refetch()} />}
      {!entries.isPending && items.length === 0 && !entries.error && <EmptyState title="No matching entries" />}
      <div>
        {items.map((e) => (
          <div key={e.id} className="entry" style={{ cursor: 'default' }}>
            <SourceIcon source={e.source} />
            <div className="grow">
              <b className="clamp" style={{ display: 'block' }}>{e.title}</b>
              <div className="small muted">{shortDate(e.occurred_at)}</div>
            </div>
            <Button size="sm" onClick={() => onInsert(e)} aria-label={`Insert ${e.title}`}>
              Insert
            </Button>
          </div>
        ))}
        {entries.hasNextPage && (
          <Button onClick={() => entries.fetchNextPage()} disabled={entries.isFetchingNextPage} style={{ width: '100%' }}>
            Load more
          </Button>
        )}
      </div>
    </Drawer>
  );
}
