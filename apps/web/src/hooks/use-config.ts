import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { AppConfig } from '@/types/api/models';

/** UI config (review sections, service metadata, career levels). Static per deployment. */
export const useConfig = () => useQuery({ queryKey: ['config'], queryFn: ({ signal }) => api<AppConfig>('/config', { signal }), staleTime: Infinity });
