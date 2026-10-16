import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { PrepResponse, SavePrepRequest, Today } from '@/types/api/models';

export const todayKeys = { today: (cycleId: number | null) => ['today', cycleId] as const, prep: (id: number) => ['meeting-prep', id] as const };

export const useToday = (cycleId: number | null) =>
  useQuery({ queryKey: todayKeys.today(cycleId), queryFn: ({ signal }) => api<Today>('/today', { signal }), placeholderData: keepPreviousData });

export const useMeetingPrep = (entryId: number | null) =>
  useQuery({ queryKey: todayKeys.prep(entryId ?? 0), queryFn: ({ signal }) => api<PrepResponse>(`/meetings/${entryId}/prep`, { signal }), enabled: entryId != null });

export function useSavePrep() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ entryId, prep }: { entryId: number; prep: SavePrepRequest }) => api<PrepResponse>(`/meetings/${entryId}/prep`, { method: 'PUT', body: prep }),
    onSuccess: (_d, v) => {
      void qc.invalidateQueries({ queryKey: todayKeys.prep(v.entryId) });
      void qc.invalidateQueries({ queryKey: ['today'] });
    },
  });
}

export const useDraftPrep = () =>
  useMutation({ mutationFn: ({ entryId, meetingGoal }: { entryId: number; meetingGoal?: string }) => api<{ content: string }>(`/meetings/${entryId}/prep/draft`, { method: 'POST', body: { meeting_goal: meetingGoal || undefined } }) });
