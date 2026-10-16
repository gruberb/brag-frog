import type { EntrySource, EntrySummary } from '@/types/api/models';

export const sourceMeta: Record<EntrySource, { label: string; abbr: string; cls: string }> = {
  github: { label: 'GitHub', abbr: 'GH', cls: 's-gh' },
  phabricator: { label: 'Phabricator', abbr: 'Ph', cls: 's-ph' },
  bugzilla: { label: 'Bugzilla', abbr: 'Bz', cls: 's-bz' },
  jira: { label: 'Jira', abbr: 'J', cls: 's-jira' },
  confluence: { label: 'Confluence', abbr: 'Cf', cls: 's-conf' },
  google_calendar: { label: 'Google Calendar', abbr: 'C', cls: 's-cal' },
  google_drive: { label: 'Google Drive', abbr: 'D', cls: 's-drive' },
  manual: { label: 'Manual', abbr: 'M', cls: 's-man' },
};

export const humanize = (s: string | null | undefined) => (s ? s.replace(/_/g, ' ') : '');

export type WorkKind = 'code' | 'review' | 'ticket' | 'other';

/** Groups synced items for the Today "open work" tabs. */
export function workKind(e: Pick<EntrySummary, 'source' | 'entry_type'>): WorkKind {
  if (/review/.test(e.entry_type)) return 'review';
  if (e.source === 'github' || e.source === 'phabricator') return 'code';
  if (e.source === 'jira' || e.source === 'bugzilla') return 'ticket';
  return 'other';
}

const MANUAL_TYPES = ['development', 'code_review', 'design_doc', 'document', 'meeting', 'workshop', 'mentoring', 'presentation', 'pairing', 'cross_team', 'interview', 'learning', 'onboarding', 'decision', 'process_improvement', 'unblocking', 'incident_response', 'investigation', 'other'];
const SYNCED_TYPES = ['pr_authored', 'pr_merged', 'pr_reviewed', 'bug_fixed', 'bug_filed', 'revision_authored', 'revision_reviewed', 'jira_completed', 'jira_story', 'jira_task', 'jira_epic', 'confluence_page', 'drive_created', 'drive_edited', 'drive_commented'];

const label = (s: string) => humanize(s).replace(/^./, (c) => c.toUpperCase());
export const manualTypeOptions = MANUAL_TYPES.map((v) => ({ value: v, label: label(v) }));
export const allTypeOptions = [...MANUAL_TYPES, ...SYNCED_TYPES].map((v) => ({ value: v, label: label(v) }));
export const reachOptions = ['team', 'cross_team', 'org', 'company'].map((v) => ({ value: v, label: label(v) }));
export const complexityOptions = ['routine', 'moderate', 'complex', 'novel'].map((v) => ({ value: v, label: label(v) }));
export const roleOptions = ['contributor', 'owner', 'lead', 'architect'].map((v) => ({ value: v, label: label(v) }));
export const sourceOptions = (Object.keys(sourceMeta) as EntrySource[]).map((v) => ({ value: v, label: sourceMeta[v].label }));
