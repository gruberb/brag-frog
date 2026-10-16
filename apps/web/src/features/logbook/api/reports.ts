import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { Report, ReportInput, ReportMeta } from '@/types/api/models';

export const reportKeys = { all: ['reports'] as const, list: (cycleId: number | null) => ['reports', cycleId] as const };

export const useReports = (cycleId: number | null, enabled = true) =>
  useQuery({ queryKey: reportKeys.list(cycleId), queryFn: ({ signal }) => api<ReportMeta[]>('/reports', { query: { cycle_id: cycleId ?? undefined }, signal }), enabled: enabled && cycleId != null });

export function useCreateReport() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (input: ReportInput) => api<Report>('/reports', { method: 'POST', body: input }), onSuccess: () => qc.invalidateQueries({ queryKey: reportKeys.all }) });
}

export function useDeleteReport() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (id: number) => api<void>(`/reports/${id}`, { method: 'DELETE' }), onSuccess: () => qc.invalidateQueries({ queryKey: reportKeys.all }) });
}

export const fetchReport = (id: number) => api<Report>(`/reports/${id}`);
