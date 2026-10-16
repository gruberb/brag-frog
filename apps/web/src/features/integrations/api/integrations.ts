import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { Integration, IntegrationInput, IntegrationsResponse, SyncStatus, TestResult } from '@/types/api/models';

export const integrationKeys = { all: ['integrations'] as const, sync: ['sync-status'] as const };

export const useIntegrations = () =>
  useQuery({ queryKey: integrationKeys.all, queryFn: async ({ signal }) => (await api<IntegrationsResponse>('/integrations', { signal })).services });

/** While a sync runs poll every 2 s, otherwise every 60 s, and not at all in a hidden tab. One shared query feeds the sidebar and Settings. */
export const useSyncStatus = () =>
  useQuery({
    queryKey: integrationKeys.sync,
    queryFn: ({ signal }) => api<SyncStatus>('/sync/status', { signal }),
    refetchInterval: (q) => (document.hidden ? false : q.state.data?.running ? 2000 : 60_000),
    refetchIntervalInBackground: false,
  });

function useIntegrationMutation<V, R>(fn: (v: V) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: integrationKeys.all });
      void qc.invalidateQueries({ queryKey: integrationKeys.sync });
      void qc.invalidateQueries({ queryKey: ['me'] });
    },
  });
}

export const useSaveIntegration = () =>
  useIntegrationMutation(({ service, input }: { service: string; input: IntegrationInput }) => api<Integration>(`/integrations/${service}`, { method: 'PUT', body: input }));
export const useTestIntegration = () => useMutation({ mutationFn: ({ service, input }: { service: string; input: IntegrationInput }) => api<TestResult>(`/integrations/${service}/test`, { method: 'POST', body: input }) });
export const useResetIntegration = () =>
  useIntegrationMutation(({ service, deleteEntries }: { service: string; deleteEntries: boolean }) =>
    api<void>(`/integrations/${service}`, { method: 'DELETE', query: { delete_entries: deleteEntries } }),
  );
export const useRestoreExcluded = () =>
  useIntegrationMutation(({ service, id }: { service: string; id: string }) => api<void>(`/integrations/${service}/excluded/${encodeURIComponent(id)}`, { method: 'DELETE' }));

export const useSync = () => {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ service, hard }: { service?: string; hard?: boolean } = {}) =>
      api<{ started: boolean }>(service ? `/sync/${service}` : '/sync', { method: 'POST', query: { hard } }),
    // 202: the run is in the background, so refresh status right away to start fast polling.
    onSuccess: () => qc.invalidateQueries({ queryKey: integrationKeys.sync }),
  });
};
