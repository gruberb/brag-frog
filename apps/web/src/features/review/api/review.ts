import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { Archive, ContentFormat, ReviewResponse } from '@/types/api/models';

export const reviewKeys = { review: (cycleId: number) => ['review', cycleId] as const, archive: (cycleId: number) => ['archive', cycleId] as const };

export const useReview = (cycleId: number | null) =>
  useQuery({ queryKey: reviewKeys.review(cycleId ?? 0), queryFn: ({ signal }) => api<ReviewResponse>(`/cycles/${cycleId}/review`, { signal }), enabled: cycleId != null });

export const useArchive = (cycleId: number | null) =>
  useQuery({ queryKey: reviewKeys.archive(cycleId ?? 0), queryFn: ({ signal }) => api<Archive>(`/cycles/${cycleId}/archive`, { signal }), enabled: cycleId != null });

export function useSaveSection(cycleId: number) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ slug, content, content_format }: { slug: string; content: string; content_format: ContentFormat }) =>
      api<void>(`/cycles/${cycleId}/review/${slug}`, { method: 'PUT', body: { content, content_format } }),
    // Patch the cache instead of refetching: a refetch would not change the editor but would flicker status pills.
    onSuccess: (_d, v) =>
      qc.setQueryData<ReviewResponse>(reviewKeys.review(cycleId), (old) =>
        old && { ...old, sections: old.sections.map((s) => (s.slug === v.slug ? { ...s, content: v.content, content_format: v.content_format } : s)) },
      ),
  });
}

export const useDraftSection = (cycleId: number) =>
  useMutation({ mutationFn: ({ slug, goal_ids }: { slug: string; goal_ids: number[] }) => api<{ markdown: string }>(`/cycles/${cycleId}/review/${slug}/draft`, { method: 'POST', body: { goal_ids } }) });
