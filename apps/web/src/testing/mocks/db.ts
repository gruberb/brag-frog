// In-memory backend used by MSW handlers, so mutations are visible to later reads in tests and in `npm run dev:mock`.
import { addDays, format, startOfISOWeek, getISOWeek, getISOWeekYear } from 'date-fns';
import type {
  Archive, Cycle, DepartmentGoal, Entry, Integration, MeetingPrep, PeopleAlias, Priority, PriorityUpdate, Profile, Report, ReviewSection, ReviewSectionConfig,
} from '@/types/api/models';

export type DbEntry = Entry & { cycle_id: number };
export type DbPriority = Priority & { cycle_id: number };

const iso = (offset: number) => addDays(new Date(), offset).toISOString();

export const sections: ReviewSectionConfig[] = [
  {
    slug: 'impact_examples', title: 'Impact examples', question: 'Describe up to 4 examples of impact you had this cycle.',
    form_question: 'Describe 1-2 examples of your work that illustrate your level of contribution and impact.', form_guidance: 'For each example, summarize:',
    form_bullets: ['The outcome or progress made', 'The behaviors demonstrated', 'What you learned'], form_tip: 'Use concise, evidence-based examples.', form_placeholder: 'Write your response…', focus_priorities: true,
  },
  { slug: 'growth', title: 'Growth', question: 'How did you grow?', form_question: 'How did you grow this cycle?', form_guidance: '', form_bullets: [], form_tip: '', form_placeholder: '', focus_priorities: false },
  { slug: 'support', title: 'Support & feedback', question: 'How did you support others?', form_question: 'How did you support others?', form_guidance: '', form_bullets: [], form_tip: '', form_placeholder: '', focus_priorities: false },
  { slug: 'looking_ahead', title: 'Looking ahead', question: 'What is next?', form_question: 'What are you focusing on next?', form_guidance: '', form_bullets: [], form_tip: '', form_placeholder: '', focus_priorities: false },
];

const SAMPLES: [string, Entry['source'], string, string | null, string[], string[]][] = [
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

export type Db = ReturnType<typeof createDb>;

function createDb() {
  const cycles: Cycle[] = [
    { id: 2, name: 'H2 2026', start_date: '2026-07-01', end_date: '2026-12-31', is_active: true, entry_count: 0, priority_count: 0 },
    { id: 1, name: 'H1 2026', start_date: '2026-01-01', end_date: '2026-06-30', is_active: false, entry_count: 0, priority_count: 0 },
  ];
  const entries: DbEntry[] = [];
  for (let i = 0; i < 90; i++) {
    const [title, source, type, repo, teams, collab] = SAMPLES[i % SAMPLES.length];
    const offset = -Math.floor(i / 2);
    const d = new Date(); d.setDate(d.getDate() + offset);
    const meeting = type === 'meeting';
    entries.push({
      id: 1000 + i, week_id: 1, cycle_id: 2, source, source_id: source === 'manual' ? null : `${source}-${i}`,
      source_url: source === 'manual' ? null : `https://example.com/${source}/${i}`,
      title: i < SAMPLES.length ? title : `${title} #${i}`, description: i % 3 === 0 ? 'Notes about what happened and why.' : '',
      entry_type: type, status: null, repository: repo, occurred_at: format(d, 'yyyy-MM-dd'), teams, collaborators: collab,
      outcome_statement: null, role: i % 4 === 0 ? 'owner' : null, reach: i % 5 === 0 ? 'team' : null, complexity: i % 6 === 0 ? 'complex' : null,
      priority_id: i % 3 === 0 ? 11 : i % 3 === 1 ? 12 : null, meeting_role: null, recurring_group: null,
      start_time: meeting ? '10:00' : null, end_time: meeting ? '10:30' : null, evidence_urls: [], impact_tags: [], decision: null, created_at: iso(offset), updated_at: iso(offset),
    });
  }
  for (let i = 0; i < 6; i++) entries.push({ ...entries[i], id: 2000 + i, cycle_id: 1, title: `H1: ${entries[i].title}`, occurred_at: format(new Date(2026, 3, 1 + i), 'yyyy-MM-dd') });

  const goals: (DepartmentGoal & { cycle_id: number })[] = [
    { id: 1, cycle_id: 2, title: 'Developer velocity', description: null, status: 'active', completed_at: null },
    { id: 2, cycle_id: 2, title: 'Platform reliability', description: null, status: 'active', completed_at: null },
  ];
  const base = { description: null, tier: null, measure_type: null, measure_start: null, measure_target: null, measure_current: null, completed_at: null, due_date: null };
  const priorities: DbPriority[] = [
    { ...base, id: 11, cycle_id: 2, title: 'Reduce CI time by 30%', status: 'active', tracking_status: 'off_track', due_date: '2026-12-15', department_goal_id: 1, measure_type: 'minutes', measure_start: 30, measure_target: 21, measure_current: 24 },
    { ...base, id: 12, cycle_id: 2, title: 'Ship workspace API v2', status: 'active', tracking_status: 'on_track', due_date: '2026-11-30', department_goal_id: 2 },
    { ...base, id: 13, cycle_id: 2, title: 'Quota alerts rollout', status: 'completed', tracking_status: 'complete', completed_at: '2026-10-09', department_goal_id: 2 },
    { ...base, id: 14, cycle_id: 2, title: 'Mentor two new hires', status: 'active', tracking_status: 'on_track', department_goal_id: null },
  ];
  const updates: PriorityUpdate[] = [
    { id: 1, priority_id: 11, tracking_status: 'off_track', measure_value: 21, comment: 'Blocked on runner capacity. Needs infra to approve 20 more nodes.', is_blocker: true, created_at: iso(-2) },
    { id: 2, priority_id: 11, tracking_status: 'off_track', measure_value: 24, comment: 'Autoscaling PR in review.', is_blocker: false, created_at: iso(-9) },
    { id: 3, priority_id: 11, tracking_status: 'on_track', measure_value: 28, comment: 'Baseline measured at 30 min, target 21.', is_blocker: false, created_at: iso(-17) },
  ];
  const reviews: Record<string, { content: string; content_format: 'markdown' | 'blocknote'; generated_at: string | null }> = {
    '2:impact_examples': { content: '## 1. Cut CI time by 45%\n\nI led the runner autoscaling effort that took median CI time from 30 to 16 minutes.', content_format: 'markdown', generated_at: null },
  };
  const preps: Record<number, MeetingPrep> = {};
  const reports: (Report & { cycle_id: number })[] = [];
  const integrations: Integration[] = [
    { service: 'github', status: 'connected', last_sync_at: iso(0), last_error: null, entry_count: 188, config: { orgs: 'mozilla' }, has_token: true, excluded: [] },
    { service: 'phabricator', status: 'connected', last_sync_at: iso(0), last_error: null, entry_count: 74, config: { base_url: 'https://phabricator.example.com' }, has_token: true, excluded: [] },
    { service: 'bugzilla', status: 'error', last_sync_at: iso(-1), last_error: 'token expired', entry_count: 12, config: {}, has_token: true, excluded: [] },
    { service: 'atlassian', status: 'connected', last_sync_at: iso(0), last_error: null, entry_count: 40, config: {}, has_token: true, excluded: [] },
    { service: 'google_calendar', status: 'connected', last_sync_at: iso(0), last_error: null, entry_count: 90, config: {}, has_token: true, excluded: [{ id: 1, label: 'Lunch' }, { id: 2, label: 'Focus block' }] },
    { service: 'google_drive', status: 'not_connected', last_sync_at: null, last_error: null, entry_count: 0, config: {}, has_token: false, excluded: [] },
  ];
  const people: PeopleAlias[] = [
    { id: 1, email: 'sam@example.com', display_name: 'Sam Okafor', team: 'Platform', relationship_type: null },
    { id: 2, email: 'priya@example.com', display_name: 'Priya Rao', team: 'Infra', relationship_type: null },
  ];
  const profile: Profile = { display_name: 'Bastian Gruber', career_level: 'senior', team: 'Platform', manager_name: 'Alex Chen', skip_level_name: null, direct_reports: null, timezone: 'Europe/Berlin', week_start: 'monday', work_start_time: '09:00', work_end_time: '17:00', wants_promotion: false };
  const archive: Archive = {
    checkins: [{ id: 1, title: 'Q2 check-in', period: '2026 Q2', content: 'Shipped storage refactor.' }],
    contribution_examples: [{ id: 1, title: 'Autoscaling', content: 'Cut CI time.', entry_titles: ['Merged runner autoscaling'] }],
    status_updates: [], last_week_reports: [],
  };
  const sync = { running: false, since: 0 };
  const w = startOfISOWeek(new Date());
  return {
    cycles, entries, goals, priorities, updates, reviews, preps, reports, integrations, people, profile, archive, sync, nextId: 5000,
    week: { id: 1, week_start: format(w, 'yyyy-MM-dd'), week_end: format(addDays(w, 4), 'yyyy-MM-dd'), iso_week: getISOWeek(new Date()), year: getISOWeekYear(new Date()) },
    hasAi: true,
    sections,
  };
}

export let db = createDb();
export const resetDb = () => { db = createDb(); };
export const sectionFor = (cycleId: number, s: ReviewSectionConfig): ReviewSection => ({ ...s, ...(db.reviews[`${cycleId}:${s.slug}`] ?? { content: '', content_format: 'blocknote', generated_at: null }) });
