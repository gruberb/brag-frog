import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { PeopleAlias, PeopleAliasInput, UpdateProfileRequest, User } from '@/types/api/models';

export const settingsKeys = { profile: ['settings', 'profile'] as const, people: ['settings', 'people'] as const };

export const useProfile = () => useQuery({ queryKey: settingsKeys.profile, queryFn: ({ signal }) => api<User>('/settings/profile', { signal }) });

export function useSaveProfile() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (p: UpdateProfileRequest) => api<User>('/settings/profile', { method: 'PUT', body: p }),
    onSuccess: (data) => {
      qc.setQueryData(settingsKeys.profile, data);
      void qc.invalidateQueries({ queryKey: ['me'] });
    },
  });
}

export const usePeople = () => useQuery({ queryKey: settingsKeys.people, queryFn: ({ signal }) => api<PeopleAlias[]>('/settings/people', { signal }) });

export function useAddPerson() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (p: PeopleAliasInput) => api<PeopleAlias[]>('/settings/people', { method: 'POST', body: p }), onSuccess: () => qc.invalidateQueries({ queryKey: settingsKeys.people }) });
}

export function useRemovePerson() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (id: number) => api<void>(`/settings/people/${id}`, { method: 'DELETE' }), onSuccess: () => qc.invalidateQueries({ queryKey: settingsKeys.people }) });
}

// The Anthropic key is stored through the integrations API under the "claude" service id.
export function useSaveAiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (token: string) => api<void>('/integrations/claude', { method: 'PUT', body: { token } }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['me'] }),
  });
}
export const useTestAiKey = () => useMutation({ mutationFn: () => api<{ ok: boolean; message: string }>('/integrations/claude/test', { method: 'POST', body: {} }) });
export function useRemoveAiKey() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: () => api<void>('/integrations/claude', { method: 'DELETE' }), onSuccess: () => qc.invalidateQueries({ queryKey: ['me'] }) });
}
