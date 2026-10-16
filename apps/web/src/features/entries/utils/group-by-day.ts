import type { EntrySummary } from '@/types/api/models';
import { dayHeading } from '@/utils/dates';

export type StreamItem = { kind: 'head'; key: string; label: string; count: number } | { kind: 'entry'; key: string; entry: EntrySummary };

export function groupByDay(entries: EntrySummary[]): StreamItem[] {
  const items: StreamItem[] = [];
  const counts = new Map<string, number>();
  entries.forEach((e) => counts.set(e.occurred_at, (counts.get(e.occurred_at) ?? 0) + 1));
  let last = '';
  for (const e of entries) {
    if (e.occurred_at !== last) {
      last = e.occurred_at;
      items.push({ kind: 'head', key: `h${last}`, label: dayHeading(last), count: counts.get(last) ?? 0 });
    }
    items.push({ kind: 'entry', key: `e${e.id}`, entry: e });
  }
  return items;
}

