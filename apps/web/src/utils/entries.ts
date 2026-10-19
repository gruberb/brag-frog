import type { Entry } from '@/types/api/models';

type SourceInfo = { label: string };

const SOURCES: Record<string, SourceInfo> = {
  github: { label: 'GitHub' },
  phabricator: { label: 'Phabricator' },
  bugzilla: { label: 'Bugzilla' },
  jira: { label: 'Jira' },
  confluence: { label: 'Confluence' },
  google_calendar: { label: 'Google Calendar' },
  google_drive: { label: 'Google Drive' },
  manual: { label: 'Manual' },
};

/** Unknown sources (new integrations) render as a neutral badge instead of crashing. */
export const sourceMeta = (source: string): SourceInfo => SOURCES[source] ?? { label: source };

export const humanize = (s: string | null | undefined) => (s ? s.replace(/_/g, ' ') : '');
const label = (s: string) => humanize(s).replace(/^./, (c) => c.toUpperCase());

export type WorkKind = 'code' | 'review' | 'ticket' | 'other';

/** Groups synced items for the Today "open work" tabs. */
export function workKind(e: Pick<Entry, 'source' | 'entry_type'>): WorkKind {
  if (/review/.test(e.entry_type)) return 'review';
  if (e.source === 'github' || e.source === 'phabricator') return 'code';
  if (e.source === 'jira' || e.source === 'bugzilla') return 'ticket';
  return 'other';
}

export const reachOptions = ['team', 'cross_team', 'org', 'company'].map((v) => ({ value: v, label: label(v) }));
export const complexityOptions = ['routine', 'moderate', 'complex', 'novel'].map((v) => ({ value: v, label: label(v) }));
export const roleOptions = ['contributor', 'owner', 'lead', 'architect'].map((v) => ({ value: v, label: label(v) }));
export const sourceOptions = Object.keys(SOURCES).map((v) => ({ value: v, label: SOURCES[v].label }));
/** Entry type categories accepted by the `category` filter. */
