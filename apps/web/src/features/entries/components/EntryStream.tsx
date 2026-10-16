import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { InfiniteData, UseInfiniteQueryResult } from '@tanstack/react-query';
import { useWindowVirtualizer } from '@tanstack/react-virtual';
import { Button } from '@/components/ui/Button';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { SkeletonRows } from '@/components/ui/Spinner';
import type { EntriesPage } from '@/types/api/models';
import { groupByDay, type StreamItem as Item } from '../utils/group-by-day';
import { EntryRow } from './EntryRow';


/** Switches to a virtual window past this many rows; below it plain DOM keeps find-in-page and a11y simple. */
const VIRTUALIZE_AFTER = 200;

type Props = {
  query: UseInfiniteQueryResult<InfiniteData<EntriesPage>>;
  onOpen: (id: number) => void;
  selectMode?: boolean;
  selected?: ReadonlySet<number>;
  onToggle?: (id: number) => void;
  emptyHint?: React.ReactNode;
};

export function EntryStream({ query, onOpen, selectMode, selected, onToggle, emptyHint }: Props) {
  const entries = useMemo(() => query.data?.pages.flatMap((p) => p.items) ?? [], [query.data]);
  const items = useMemo(() => groupByDay(entries), [entries]);
  const sentinel = useRef<HTMLDivElement>(null);
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = query;

  // Infinite scroll: load the next page when the sentinel nears the viewport.
  useEffect(() => {
    const el = sentinel.current;
    if (!el || !hasNextPage || typeof IntersectionObserver === 'undefined') return;
    const io = new IntersectionObserver((hits) => hits[0]?.isIntersecting && !isFetchingNextPage && void fetchNextPage(), { rootMargin: '400px' });
    io.observe(el);
    return () => io.disconnect();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage, items.length]);

  if (query.isPending) return <SkeletonRows rows={6} h={46} />;
  if (query.error) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  if (entries.length === 0) return <EmptyState title="No entries match">{emptyHint ?? 'Try clearing a filter.'}</EmptyState>;

  const render = (it: Item) =>
    it.kind === 'head' ? (
      <div key={it.key} className="day-h sticky">
        <span>{it.label}</span>
        <span className="count">{it.count}</span>
      </div>
    ) : (
      <EntryRow key={it.key} entry={it.entry} onOpen={onOpen} selectMode={selectMode} selected={selected?.has(it.entry.id)} onToggle={onToggle} />
    );

  return (
    <div style={{ opacity: query.isPlaceholderData ? 0.6 : 1 }}>
      {items.length > VIRTUALIZE_AFTER ? <Virtual items={items} render={render} /> : items.map(render)}
      <div ref={sentinel} />
      {hasNextPage && (
        <div style={{ textAlign: 'center', margin: '12px 0' }}>
          <Button onClick={() => void fetchNextPage()} disabled={isFetchingNextPage}>
            {isFetchingNextPage ? 'Loading…' : 'Load more'}
          </Button>
        </div>
      )}
    </div>
  );
}

function Virtual({ items, render }: { items: Item[]; render: (i: Item) => React.ReactNode }) {
  // The list's offset from the page top is read when it mounts, via a callback ref (not during render).
  const [margin, setMargin] = useState(0);
  const parent = useCallback((el: HTMLDivElement | null) => { if (el) setMargin(el.offsetTop); }, []);
  const v = useWindowVirtualizer({ count: items.length, estimateSize: (i) => (items[i].kind === 'head' ? 56 : 54), overscan: 12, scrollMargin: margin });
  return (
    <div ref={parent} style={{ height: v.getTotalSize(), position: 'relative' }}>
      {v.getVirtualItems().map((row) => (
        <div key={row.key} data-index={row.index} ref={v.measureElement} style={{ position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${row.start - margin}px)` }}>
          {render(items[row.index])}
        </div>
      ))}
    </div>
  );
}
