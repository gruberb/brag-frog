import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { DepartmentGoal, ImportResult, PrioritiesResponse, Priority, PriorityInput, PriorityUpdate, PriorityUpdateInput } from '@/types/api/models';

// Same key shape as hooks/use-priority-options, so pickers elsewhere share this cache entry.
export const priorityKeys = { all: ['priorities'] as const, list: (cycleId: number | null | undefined) => ['priorities', cycleId] as const, updates: (id: number) => ['priority-updates', id] as const };

export const usePriorities = (cycleId: number | null) =>
  useQuery({ queryKey: priorityKeys.list(cycleId), queryFn: ({ signal }) => api<PrioritiesResponse>('/priorities', { query: { cycle_id: cycleId ?? undefined }, signal }), enabled: cycleId != null });

export const usePriorityUpdates = (id: number | null) =>
  useQuery({ queryKey: priorityKeys.updates(id ?? 0), queryFn: ({ signal }) => api<PriorityUpdate[]>(`/priorities/${id}/updates`, { signal }), enabled: id != null });

function useRefresh() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: priorityKeys.all });
    void qc.invalidateQueries({ queryKey: ['today'] });
  };
}

export function useCreatePriority() {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (input: PriorityInput) => api<Priority>('/priorities', { method: 'POST', body: input }), onSuccess: refresh });
}

export function useUpdatePriority() {
  const qc = useQueryClient();
  const refresh = useRefresh();
  return useMutation({
    mutationFn: ({ id, input }: { id: number; input: PriorityInput }) => api<Priority>(`/priorities/${id}`, { method: 'PUT', body: input }),
    // Status changes are optimistic: the row moves between Active and Done immediately.
    onMutate: async ({ id, input }) => {
      await qc.cancelQueries({ queryKey: priorityKeys.all });
      const snap = qc.getQueriesData<PrioritiesResponse>({ queryKey: priorityKeys.all });
      const patch = <T extends { id: number }>(xs: T[]) => xs.map((p) => (p.id === id ? { ...p, ...input } : p));
      for (const [k, d] of snap) if (d) qc.setQueryData<PrioritiesResponse>(k, { ...d, goals: d.goals.map((g) => ({ ...g, priorities: patch(g.priorities) })), unassigned: patch(d.unassigned) });
      return snap;
    },
    onError: (_e, _v, snap) => snap?.forEach(([k, d]) => qc.setQueryData(k, d)),
    onSettled: refresh,
  });
}

export function useDeletePriority() {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (id: number) => api<void>(`/priorities/${id}`, { method: 'DELETE' }), onSuccess: refresh });
}

export function usePostUpdate(priorityId: number) {
  const qc = useQueryClient();
  const refresh = useRefresh();
  return useMutation({
    mutationFn: (input: PriorityUpdateInput) => api<PriorityUpdate>(`/priorities/${priorityId}/updates`, { method: 'POST', body: input }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: priorityKeys.updates(priorityId) });
      refresh();
    },
  });
}

export function useCreateGoal() {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (input: { title: string; cycle_id: number }) => api<DepartmentGoal>('/goals', { method: 'POST', body: input }), onSuccess: refresh });
}

export function useCompleteGoal() {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (id: number) => api<DepartmentGoal>(`/goals/${id}`, { method: 'PUT', body: { status: 'completed' } }), onSuccess: refresh });
}

export function useImportLattice() {
  const refresh = useRefresh();
  return useMutation({
    mutationFn: ({ file, cycleId }: { file: File; cycleId: number }) => {
      const fd = new FormData();
      fd.append('csv_file', file);
      fd.append('cycle_id', String(cycleId));
      return api<ImportResult>('/priorities/import', { method: 'POST', body: fd });
    },
    onSuccess: refresh,
  });
}
