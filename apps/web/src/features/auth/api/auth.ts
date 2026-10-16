import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { Me, PublicConfig } from '@/types/api/models';

export const authKeys = { me: ['me'] as const, publicConfig: ['public-config'] as const };

export const useMe = () => useQuery({ queryKey: authKeys.me, queryFn: ({ signal }) => api<Me>('/me', { signal }), staleTime: 60_000 });

export const usePublicConfig = () =>
  useQuery({ queryKey: authKeys.publicConfig, queryFn: ({ signal }) => api<PublicConfig>('/public-config', { signal }), staleTime: 5 * 60_000 });
