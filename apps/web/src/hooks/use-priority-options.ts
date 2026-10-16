import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { PrioritiesResponse } from '@/types/api/models';

/**
 * Read-side view of the priorities query for pickers in other features (entry drawer, filters, evidence).
 * Shares the ['priorities', cycleId] key with features/priorities, so one fetch serves both.
 */
export function usePriorityOptions(cycleId: number | null | undefined) {
  const q = useQuery({
    queryKey: ['priorities', cycleId],
    queryFn: ({ signal }) => api<PrioritiesResponse>('/priorities', { query: { cycle_id: cycleId ?? undefined }, signal }),
    enabled: cycleId != null,
  });
  const goals = q.data?.goals.map((g) => ({ value: String(g.goal.id), label: g.goal.title })) ?? [];
  const priorities = [...(q.data?.goals.flatMap((g) => g.priorities) ?? []), ...(q.data?.unassigned ?? [])].map((p) => ({ value: String(p.id), label: p.title }));
  return { goals, priorities, isLoading: q.isLoading };
}
