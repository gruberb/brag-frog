// In-memory backend used by MSW handlers, so mutations are visible to later reads in tests and in `npm run dev:mock`.
// Every value is typed with the generated ts-rs DTOs, so a contract change breaks compilation here first.
import { addDays, format, startOfISOWeek, getISOWeek, getISOWeekYear } from 'date-fns';
import type { ArchiveResponse } from '@/types/api/ArchiveResponse';
import type { ConfigField } from '@/types/api/ConfigField';
import type { CycleSummaryDto } from '@/types/api/CycleSummaryDto';
import type { EntryDto } from '@/types/api/EntryDto';
import type { GoalDto } from '@/types/api/GoalDto';
import type { IntegrationDto } from '@/types/api/IntegrationDto';
import type { PersonDto } from '@/types/api/PersonDto';
import type { PriorityDto } from '@/types/api/PriorityDto';
import type { PriorityUpdateDto } from '@/types/api/PriorityUpdateDto';
import type { ReportDto } from '@/types/api/ReportDto';
import type { ReviewSectionConfig } from '@/types/api/ReviewSectionConfig';
import type { ReviewSectionDto } from '@/types/api/ReviewSectionDto';
import type { UserDto } from '@/types/api/UserDto';

export type DbEntry = EntryDto & { cycle_id: number };

const iso = (offset: number) => addDays(new Date(), offset).toISOString();

const section = (slug: string, title: string, question: string, extra: Partial<ReviewSectionConfig> = {}): ReviewSectionConfig => ({
  slug, title, question, form_question_number: null, form_required: false, form_question: question, form_guidance: null, form_bullets: [], form_tip: null, form_placeholder: null,
  focus_priorities: false, focus_priority_help: null, ...extra,
});

export const sections: ReviewSectionConfig[] = [
  section('impact_examples', 'Impact examples', 'Describe up to 4 examples of impact you had this cycle.', {
    form_question_number: '1', form_required: true, form_guidance: 'For each example, summarize:', focus_priorities: true,
    form_bullets: ['The outcome or progress made', 'The behaviors demonstrated', 'What you learned'], form_tip: 'Use concise, evidence-based examples.',
  }),
  section('growth', 'Growth', 'How did you grow?'),
  section('support', 'Support & feedback', 'How did you support others?'),
  section('looking_ahead', 'Looking ahead', 'What is next?'),
];

const SAMPLES: [string, string, string, string | null, string[], string[]][] = [
  ['Merged runner autoscaling', 'github', 'pr_merged', 'ci-tools', ['Platform'], ['Sam']],
  ['Add cache warmup job', 'github', 'pr_authored', 'ci-tools', ['Platform'], []],
  ['Reviewed D281944 Refactor storage layer', 'phabricator', 'revision_reviewed', 'workspace', ['Platform'], ['Priya']],
  ['D281977 Quota API', 'phabricator', 'revision_authored', 'workspace', ['Platform'], []],
  ['Bug 1982244 Intermittent timeout', 'bugzilla', 'bug_fixed', null, ['Infra'], []],
  ['PLAT-1203 Quota alerts', 'jira', 'jira_completed', null, ['Platform'], ['Jordan']],
  ['Technical Plan: Image Service v2', 'google_drive', 'drive_edited', null, ['Platform'], []],
  ['RFC: External Logo vendor', 'google_drive', 'drive_created', null, [], []],
  ['DISCO: Grooming', 'google_calendar', 'meeting', null, ['Platform'], ['Sam', 'Priya']],
  ['1:1 with Sam', 'google_calendar', 'meeting', null, ['Platform'], ['Sam']],
  ['Mentored new hire on storage internals', 'manual', 'mentoring', null, ['Platform'], ['Jordan']],
  ['Decided to adopt gradual rollouts', 'manual', 'decision', null, ['Platform', 'Infra'], ['Sam']],
  ['feat(enterprise): publish enterprise-beta image', 'github', 'pr_reviewed', 'enterprise', ['Security'], []],
  ['Incident: runner pool exhausted', 'manual', 'incident_response', null, ['Infra'], ['Priya', 'Jordan']],
];

const field = (name: string, label: string, extra: Partial<ConfigField> = {}): ConfigField => ({ name, label, field_type: 'text', placeholder: null, default_value: '', value: '', ...extra });

function integration(service: string, display_name: string, extra: Partial<IntegrationDto> = {}): IntegrationDto {
  return {
    service, display_name, is_enabled: true, has_token: true, last_sync_at: iso(0), last_sync_status: 'success', last_sync_error: null, token_only: false, oauth_connect: false,
    token_url: '', note: '', fields: [], excluded_files: [], excluded_events: [], entry_count: 0, ...extra,
  };
}

const prio = (id: number, title: string, extra: Partial<PriorityDto> = {}): PriorityDto => ({
  id, cycle_id: 2, title, status: 'active', color: null, sort_order: id, scope: null, started_at: null, completed_at: null, impact_narrative: null, department_goal_id: null, created_at: iso(-60),
  priority_level: null, measure_type: null, measure_start: null, measure_target: null, measure_current: null, description: null, tracking_status: 'on_track', due_date: null, tier: null,
  entry_count: 0, latest_update: null, ...extra,
});

export type Db = ReturnType<typeof createDb>;

function createDb() {
  const cycles: CycleSummaryDto[] = [
    { id: 2, name: 'H2 2026', start_date: '2026-07-01', end_date: '2026-12-31', is_active: true, entry_count: 0, priority_count: 0 },
    { id: 1, name: 'H1 2026', start_date: '2026-01-01', end_date: '2026-06-30', is_active: false, entry_count: 0, priority_count: 0 },
  ];
  const entries: DbEntry[] = [];
  for (let i = 0; i < 90; i++) {
    const [title, source, type, repo, teams, collab] = SAMPLES[i % SAMPLES.length];
    const offset = -Math.floor(i / 2);
    const meeting = type === 'meeting';
    entries.push({
      id: 1000 + i, week_id: 1, cycle_id: 2, source, source_id: source === 'manual' ? null : `${source}-${i}`, source_url: source === 'manual' ? null : `https://example.com/${source}/${i}`,
      title: i < SAMPLES.length ? title : `${title} #${i}`, description: i % 3 === 0 ? 'Notes about what happened and why.' : null, entry_type: type, status: null, repository: repo,
      occurred_at: format(addDays(new Date(), offset), 'yyyy-MM-dd'), teams, collaborators: collab, outcome_statement: null, role: i % 4 === 0 ? 'owner' : null, reach: i % 5 === 0 ? 'team' : null,
      complexity: i % 6 === 0 ? 'complex' : null, priority_id: i % 3 === 0 ? 11 : i % 3 === 1 ? 12 : null, meeting_role: null, recurring_group: null,
      start_time: meeting ? '10:00' : null, end_time: meeting ? '10:30' : null, evidence_urls: null, impact_tags: null, decision: null, is_synced: source !== 'manual',
      created_at: iso(offset), updated_at: iso(offset),
    });
  }
  for (let i = 0; i < 6; i++) entries.push({ ...entries[i], id: 2000 + i, cycle_id: 1, title: `H1: ${entries[i].title}`, occurred_at: format(new Date(2026, 3, 1 + i), 'yyyy-MM-dd') });

  const goals: GoalDto[] = [
    { id: 1, cycle_id: 2, title: 'Developer velocity', description: null, status: 'active', sort_order: 1, source: 'manual', created_at: iso(-70) },
    { id: 2, cycle_id: 2, title: 'Platform reliability', description: null, status: 'active', sort_order: 2, source: 'manual', created_at: iso(-70) },
  ];
  const priorities: PriorityDto[] = [
    prio(11, 'Reduce CI time by 30%', { tracking_status: 'off_track', due_date: '2026-12-15', department_goal_id: 1, measure_type: 'minutes', measure_start: 30, measure_target: 21, measure_current: 24 }),
    prio(12, 'Ship workspace API v2', { due_date: '2026-11-30', department_goal_id: 2 }),
    prio(13, 'Quota alerts rollout', { status: 'completed', tracking_status: 'complete', completed_at: '2026-10-09T00:00:00Z', department_goal_id: 2 }),
    prio(14, 'Mentor two new hires'),
  ];
  const updates: PriorityUpdateDto[] = [
    { id: 1, priority_id: 11, tracking_status: 'off_track', measure_value: 21, comment: 'Blocked on runner capacity. Needs infra to approve 20 more nodes.', is_blocker: true, tradeoff_text: null, created_at: iso(-2) },
    { id: 2, priority_id: 11, tracking_status: 'off_track', measure_value: 24, comment: 'Autoscaling PR in review.', is_blocker: false, tradeoff_text: null, created_at: iso(-9) },
    { id: 3, priority_id: 11, tracking_status: 'on_track', measure_value: 28, comment: 'Baseline measured at 30 min, target 21.', is_blocker: false, tradeoff_text: null, created_at: iso(-17) },
  ];
  const reviews: Record<string, { content: string; content_format: string; generated_at: string | null }> = {
    '2:impact_examples': { content: '## 1. Cut CI time by 45%\n\nI led the runner autoscaling effort that took median CI time from 30 to 16 minutes.', content_format: 'markdown', generated_at: null },
  };
  const preps: Record<number, { notes: string | null; meeting_goal: string | null; doc_urls: string | null; updated_at: string }> = { 7002: { notes: 'Talk about growth', meeting_goal: null, doc_urls: null, updated_at: iso(-1) } };
  const reports: ReportDto[] = [];
  const integrations: IntegrationDto[] = [
    integration('github', 'GitHub', { entry_count: 188, token_url: 'https://github.com/settings/tokens/new', note: 'Create a classic token.', fields: [field('org', 'Organizations', { placeholder: 'my-org', value: 'acme' }), field('sync_pr_authored', 'Sync authored PRs', { field_type: 'checkbox', default_value: true, value: true })] }),
    integration('phabricator', 'Phabricator', { entry_count: 74, fields: [field('base_url', 'Base URL', { placeholder: 'https://phabricator.example.com', value: 'https://phabricator.example.com' })] }),
    integration('bugzilla', 'Bugzilla', { entry_count: 12, last_sync_status: 'error', last_sync_error: 'token expired', fields: [field('base_url', 'Base URL'), field('email', 'Email')] }),
    integration('atlassian', 'Atlassian', { entry_count: 40, note: 'Syncs Jira and Confluence.', fields: [field('base_url', 'Base URL', { placeholder: 'https://your-org.atlassian.net', default_value: 'https://example.atlassian.net', value: 'https://example.atlassian.net' }), field('email', 'Email')] }),
    integration('google_calendar', 'Google Calendar', { entry_count: 90, oauth_connect: true, has_token: false, excluded_events: [{ id: 'ev1', title: 'Lunch' }, { id: 'ev2', title: 'Focus block' }] }),
    integration('claude', 'Claude', { token_only: true, last_sync_at: null, last_sync_status: null }),
    integration('google_drive', 'Google Drive', { is_enabled: false, has_token: false, oauth_connect: true, last_sync_at: null, last_sync_status: null }),
  ];
  const people: PersonDto[] = [
    { id: 1, email: 'sam@example.com', display_name: 'Sam Okafor', team: 'Platform' },
    { id: 2, email: 'priya@example.com', display_name: 'Priya Rao', team: 'Infra' },
  ];
  const profile: UserDto = {
    id: 1, email: 'bgruber@example.com', name: 'Bastian Gruber', avatar_url: null, role: 'senior', wants_promotion: false, display_name: 'Bastian', team: 'Platform', manager_name: 'Alex Chen',
    skip_level_name: null, direct_reports: null, timezone: 'Europe/Berlin', week_start: 'monday', work_start_time: '09:00', work_end_time: '17:00',
  };
  const archive: ArchiveResponse = {
    cycle_id: 2,
    checkins: [{ id: 1, quarter: 'Q2', year: 2026, highlights_impact: 'Shipped storage refactor.', learnings_adjustments: null, growth_development: null, support_feedback: null, looking_ahead: null, updated_at: iso(-100) }],
    contribution_examples: [{ id: 1, title: 'Autoscaling', outcome: 'Cut CI time.', behaviors: null, impact_level: 'team', learnings: null, assessment_type: null, status: 'final', entries: [{ id: 1000, title: 'Merged runner autoscaling' }] }],
    status_updates: [], last_week_reports: [],
  };
  const sync = { running: false, since: 0 };
  const w = startOfISOWeek(new Date());
  return {
    cycles, entries, goals, priorities, updates, reviews, preps, reports, integrations, people, profile, archive, sync, nextId: 5000,
    week: { id: 1, iso_week: getISOWeek(new Date()), year: getISOWeekYear(new Date()), start_date: format(w, 'yyyy-MM-dd'), end_date: format(addDays(w, 6), 'yyyy-MM-dd') },
    hasAi: true,
    sections,
  };
}

export let db = createDb();
export const resetDb = () => { db = createDb(); };

export const sectionFor = (cycleId: number, s: ReviewSectionConfig): ReviewSectionDto => ({ template: '1. Example headline\n\n**Status:** done', ...s, ...(db.reviews[`${cycleId}:${s.slug}`] ?? { content: null, content_format: 'blocknote', generated_at: null }) });
