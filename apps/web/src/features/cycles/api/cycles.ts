import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { Cycle, CycleBundle, CycleSummary } from '@/types/api/models';

export const cycleKeys = { all: ['cycles'] as const, bundle: (id: number) => ['cycles', id, 'bundle'] as const };

export const useCycles = () => useQuery({ queryKey: cycleKeys.all, queryFn: ({ signal }) => api<CycleSummary[]>('/cycles', { signal }) });

/** A cycle change touches nearly every cached view, so everything is invalidated. */
function useCycleMutation<V, R>(fn: (v: V) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({ mutationFn: fn, onSuccess: () => qc.invalidateQueries() });
}

type CycleInput = { name: string; start_date: string; end_date: string; activate: boolean };

/** The API creates inactive cycles; activating is a second call so the choice stays explicit. */
export const useCreateCycle = () =>
  useCycleMutation(async ({ activate, ...body }: CycleInput) => {
    const cycle = await api<Cycle>('/cycles', { method: 'POST', body });
    if (activate) await api<Cycle>(`/cycles/${cycle.id}/activate`, { method: 'POST' });
    return cycle;
  });
export const useActivateCycle = () => useCycleMutation((id: number) => api<Cycle>(`/cycles/${id}/activate`, { method: 'POST' }));
export const useDeleteCycle = () =>
  useCycleMutation(({ id, confirm_name }: { id: number; confirm_name: string }) => api<void>(`/cycles/${id}`, { method: 'DELETE', body: { confirm_name } }));

export const fetchCycleBundle = (id: number) => api<CycleBundle>(`/cycles/${id}/bundle`);
