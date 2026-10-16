import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { TrendRange, Trends } from '@/types/api/models';

export const useTrends = (cycleId: number | null, range: TrendRange) =>
  useQuery({
    queryKey: ['trends', cycleId, range],
    queryFn: ({ signal }) => api<Trends>('/trends', { query: { cycle_id: cycleId ?? undefined, range }, signal }),
    enabled: cycleId != null,
    // Keep the old range on screen while the next one loads.
    placeholderData: keepPreviousData,
  });
