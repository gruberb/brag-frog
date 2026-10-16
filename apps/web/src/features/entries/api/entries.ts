import { keepPreviousData, useInfiniteQuery, useMutation, useQuery, useQueryClient, type InfiniteData, type QueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { BulkRequest, ClassifyRequest, EntriesPage, Entry, EntryFacets, EntryFilters, EntryInput } from '@/types/api/models';

const PAGE = 50;

export const entryKeys = {
  all: ['entries'] as const,
  list: (cycleId: number | null, filters: EntryFilters) => ['entries', cycleId, filters] as const,
  facets: (cycleId: number | null) => ['entry-facets', cycleId] as const,
  detail: (id: number) => ['entry', id] as const,
};

export function useEntries(cycleId: number | null, filters: EntryFilters, enabled = true) {
  return useInfiniteQuery({
    queryKey: entryKeys.list(cycleId, filters),
    queryFn: ({ pageParam, signal }) => api<EntriesPage>('/entries', { query: { ...filters, cycle_id: cycleId ?? undefined, cursor: pageParam || undefined, limit: PAGE }, signal }),
    initialPageParam: '',
    getNextPageParam: (last) => last.next_cursor ?? undefined,
    placeholderData: keepPreviousData,
    enabled: enabled && cycleId != null,
  });
}

export const useEntryFacets = (cycleId: number | null) =>
  useQuery({ queryKey: entryKeys.facets(cycleId), queryFn: ({ signal }) => api<EntryFacets>('/entries/facets', { query: { cycle_id: cycleId ?? undefined }, signal }), enabled: cycleId != null });

export const useEntry = (id: number | null) =>
  useQuery({ queryKey: entryKeys.detail(id ?? 0), queryFn: ({ signal }) => api<Entry>(`/entries/${id}`, { signal }), enabled: id != null });

type Pages = InfiniteData<EntriesPage>;

const SUMMARY_KEYS = ['title', 'entry_type', 'occurred_at', 'teams', 'collaborators', 'priority_id', 'reach', 'complexity', 'role', 'description'] as const;
/** Only the fields shown in list rows are patched optimistically; the rest arrives with the refetch. */
function toSummaryPatch(input: EntryInput): Partial<Entry> {
  const out: Record<string, unknown> = {};
  for (const k of SUMMARY_KEYS) if (k in input) out[k] = input[k];
  return out as Partial<Entry>;
}

/** Applies `fn` to every cached entries list and returns the previous values for rollback. */
function patchLists(qc: QueryClient, fn: (p: Pages) => Pages) {
  const snapshot = qc.getQueriesData<Pages>({ queryKey: entryKeys.all });
  for (const [key, data] of snapshot) if (data) qc.setQueryData<Pages>(key, fn(data));
  return snapshot;
}
const rollback = (qc: QueryClient, snap: [readonly unknown[], Pages | undefined][]) => snap.forEach(([k, d]) => qc.setQueryData(k, d));

function refreshEntryViews(qc: QueryClient) {
  void qc.invalidateQueries({ queryKey: entryKeys.all });
  void qc.invalidateQueries({ queryKey: ['entry-facets'] });
  void qc.invalidateQueries({ queryKey: ['today'] });
}

export function useCreateEntry() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: EntryInput) => api<Entry>('/entries', { method: 'POST', body: input }),
    onMutate: async (input) => {
      await qc.cancelQueries({ queryKey: entryKeys.all });
      // Optimistic insert at the top of every list; the refetch below replaces the temporary row.
      const now = new Date().toISOString();
      const temp: Entry = {
        id: -Date.now(), week_id: 0, source: 'manual', source_id: null, source_url: input.source_url ?? null, title: input.title, description: input.description ?? null, entry_type: input.entry_type,
        status: null, repository: null, occurred_at: input.occurred_at ?? now.slice(0, 10), teams: input.teams ?? [], collaborators: input.collaborators ?? [], outcome_statement: null,
        role: input.role ?? null, reach: input.reach ?? null, complexity: input.complexity ?? null, priority_id: input.priority_id ?? null, meeting_role: null, recurring_group: null,
        start_time: null, end_time: null, evidence_urls: null, impact_tags: null, decision: null, is_synced: false, created_at: now, updated_at: now,
      };
      return patchLists(qc, (d) => ({ ...d, pages: d.pages.map((p, i) => (i === 0 ? { ...p, items: [temp, ...p.items], total_matching: p.total_matching + 1 } : p)) }));
    },
    onError: (_e, _v, snap) => snap && rollback(qc, snap),
    onSettled: () => refreshEntryViews(qc),
  });
}

export function useUpdateEntry() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: number; input: EntryInput }) => api<Entry>(`/entries/${id}`, { method: 'PUT', body: input }),
    onMutate: async ({ id, input }) => {
      await qc.cancelQueries({ queryKey: entryKeys.all });
      const summaryPatch = toSummaryPatch(input);
      return patchLists(qc, (d) => ({ ...d, pages: d.pages.map((p) => ({ ...p, items: p.items.map((e) => (e.id === id ? { ...e, ...summaryPatch } : e)) })) }));
    },
    onError: (_e, _v, snap) => snap && rollback(qc, snap),
    onSettled: (_d, _e, v) => {
      void qc.invalidateQueries({ queryKey: entryKeys.detail(v.id) });
      refreshEntryViews(qc);
    },
  });
}

export function useDeleteEntry() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (id: number) => api<void>(`/entries/${id}`, { method: 'DELETE' }), onSuccess: () => refreshEntryViews(qc) });
}

export function useBulkUpdate() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (v: BulkRequest) => api<{ updated: number }>('/entries/bulk', { method: 'POST', body: v }), onSuccess: () => refreshEntryViews(qc) });
}

export function useClassifyEntry() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, ...body }: ClassifyRequest & { id: number }) => api<void>(`/entries/${id}/classify`, { method: 'POST', body }),
    onSuccess: () => refreshEntryViews(qc),
  });
}

export function useExcludeEntry() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (id: number) => api<void>(`/entries/${id}/exclude`, { method: 'POST' }), onSuccess: () => refreshEntryViews(qc) });
}
