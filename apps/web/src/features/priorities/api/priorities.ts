import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { CreatePriorityRequest, Goal, ImportResult, PrioritiesResponse, Priority, PriorityUpdate, PostUpdateRequest, UpdatePriorityRequest } from '@/types/api/models';

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
  return useMutation({ mutationFn: (input: CreatePriorityRequest) => api<Priority>('/priorities', { method: 'POST', body: input }), onSuccess: refresh });
}

export function useUpdatePriority() {
  const qc = useQueryClient();
  const refresh = useRefresh();
  return useMutation({
    mutationFn: ({ id, input }: { id: number; input: UpdatePriorityRequest }) => api<Priority>(`/priorities/${id}`, { method: 'PUT', body: input }),
    // Status changes are optimistic: the row moves between Active and Done immediately.
    onMutate: async ({ id, input }) => {
      await qc.cancelQueries({ queryKey: priorityKeys.all });
      const snap = qc.getQueriesData<PrioritiesResponse>({ queryKey: priorityKeys.all });
      const patch = (xs: Priority[]) => xs.map((p) => (p.id === id ? ({ ...p, ...input } as Priority) : p));
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
    mutationFn: (input: PostUpdateRequest) => api<PriorityUpdate>(`/priorities/${priorityId}/updates`, { method: 'POST', body: input }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: priorityKeys.updates(priorityId) });
      refresh();
    },
  });
}

export function useCreateGoal() {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (input: { title: string; cycle_id: number }) => api<Goal>('/goals', { method: 'POST', body: input }), onSuccess: refresh });
}

export function useCompleteGoal() {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (id: number) => api<Goal>(`/goals/${id}`, { method: 'PUT', body: { status: 'completed' } }), onSuccess: refresh });
}

export function useImportLattice() {
  const refresh = useRefresh();
  return useMutation({
    mutationFn: ({ file, cycleId }: { file: File; cycleId: number }) => {
      const fd = new FormData();
      fd.append('csv_file', file);
      return api<ImportResult>('/priorities/import', { method: 'POST', body: fd, query: { cycle_id: cycleId } });
    },
    onSuccess: refresh,
  });
}
