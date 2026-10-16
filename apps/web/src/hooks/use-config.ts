import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { AppConfig } from '@/types/api/models';

/** UI config (review sections, service metadata, career levels). Static per deployment. */
export const useConfig = () => useQuery({ queryKey: ['config'], queryFn: ({ signal }) => api<AppConfig>('/config', { signal }), staleTime: Infinity });

/** Atlassian base URL (for Jira key links): the saved or default `base_url` field of the atlassian service. */
export function useJiraBase(): string | null {
  const field = useConfig().data?.services.find((s) => s.service === 'atlassian')?.fields.find((f) => f.name === 'base_url');
  const v = field?.value || field?.default_value;
  return typeof v === 'string' && v.startsWith('http') ? v : null;
}
