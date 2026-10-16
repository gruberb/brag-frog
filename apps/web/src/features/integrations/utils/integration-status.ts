import type { Integration } from '@/types/api/models';

export type IntegrationState = 'connected' | 'error' | 'not_connected';

export function integrationState(i: Integration): IntegrationState {
  if (!i.is_enabled) return 'not_connected';
  return i.last_sync_status === 'error' || i.last_sync_error ? 'error' : 'connected';
}

/** Every key of the generated `IntegrationInput` must be present; null means "not provided". */
export const emptyInput = (): import('@/types/api/models').IntegrationInput => ({
  token: null, username: null, email: null, org: null, base_url: null, token_expires: null, sync_pr_authored: null, sync_pr_merged: null, sync_pr_reviewed: null,
  sync_pr_development: null, sync_jira_completed: null, sync_jira_in_progress: null, sync_jira_created: null,
});
