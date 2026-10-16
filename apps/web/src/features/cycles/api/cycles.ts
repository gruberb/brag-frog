import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { Cycle, CycleBundle, CycleInput } from '@/types/api/models';

export const cycleKeys = { all: ['cycles'] as const, bundle: (id: number) => ['cycles', id, 'bundle'] as const };

export const useCycles = () => useQuery({ queryKey: cycleKeys.all, queryFn: ({ signal }) => api<Cycle[]>('/cycles', { signal }) });

/** A cycle change touches nearly every cached view, so everything is invalidated. */
function useCycleMutation<V, R>(fn: (v: V) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({ mutationFn: fn, onSuccess: () => qc.invalidateQueries() });
}

export const useCreateCycle = () => useCycleMutation((input: CycleInput) => api<Cycle>('/cycles', { method: 'POST', body: input }));
export const useActivateCycle = () => useCycleMutation((id: number) => api<void>(`/cycles/${id}/activate`, { method: 'POST' }));
export const useDeleteCycle = () =>
  useCycleMutation(({ id, confirm_name }: { id: number; confirm_name: string }) => api<void>(`/cycles/${id}`, { method: 'DELETE', body: { confirm_name } }));

export const fetchCycleBundle = (id: number) => api<CycleBundle>(`/cycles/${id}/bundle`);
