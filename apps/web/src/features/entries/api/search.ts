import { api } from '@/lib/api-client';
import type { EntriesPage } from '@/types/api/models';

/** Entry lookup for the `[[` reference menu in the editor. Empty query returns the most recent entries. */
export async function searchEntries(cycleId: number, query: string) {
  const page = await api<EntriesPage>('/entries', { query: { cycle_id: cycleId, q: query.trim() || undefined, limit: 8 } });
  return page.items.map((e) => ({ id: e.id, title: e.title, subtext: `${e.occurred_at} · ${e.entry_type.replace(/_/g, ' ')}` }));
}
