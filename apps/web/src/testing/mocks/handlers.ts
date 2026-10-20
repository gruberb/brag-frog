import { http, HttpResponse, delay } from 'msw';
import { addDays, format, parseISO, startOfISOWeek, subWeeks } from 'date-fns';
import type { BulkRequest } from '@/types/api/BulkRequest';
import type { ConfigResponse } from '@/types/api/ConfigResponse';
import type { CreatePriorityRequest } from '@/types/api/CreatePriorityRequest';
import type { CreateReportRequest } from '@/types/api/CreateReportRequest';
import type { CycleDto } from '@/types/api/CycleDto';
import type { CycleSummaryDto } from '@/types/api/CycleSummaryDto';
import type { EntryDto } from '@/types/api/EntryDto';
import type { EntryInput } from '@/types/api/EntryInput';
import type { EntryTypeDto } from '@/types/api/EntryTypeDto';
import type { IntegrationInput } from '@/types/api/IntegrationInput';
import type { MeetingDto } from '@/types/api/MeetingDto';
import type { PostUpdateRequest } from '@/types/api/PostUpdateRequest';
import type { PrepResponse } from '@/types/api/PrepResponse';
import type { PriorityDto } from '@/types/api/PriorityDto';
import type { ReportKeyEntry } from '@/types/api/ReportKeyEntry';
import type { ReportStats } from '@/types/api/ReportStats';
import type { SavePrepRequest } from '@/types/api/SavePrepRequest';
import type { SyncStatus } from '@/types/api/SyncStatus';
import type { TodayResponse } from '@/types/api/TodayResponse';
import type { TrendRange } from '@/types/api/TrendRange';
import type { TrendsResponse } from '@/types/api/TrendsResponse';
import type { UpdatePriorityRequest } from '@/types/api/UpdatePriorityRequest';
import type { UpdateProfileRequest } from '@/types/api/UpdateProfileRequest';
import { db, sectionFor, type DbEntry } from './db';

const BASE = '/api/v1';
const err = (status: number, code: string, message: string) => HttpResponse.json({ error: { code, message } }, { status });
const noContent = () => new HttpResponse(null, { status: 204 });
const num = (v: string | null) => (v ? Number(v) : undefined);
const dto = ({ cycle_id: _c, ...e }: DbEntry): EntryDto => (void _c, e);

// Mirrors `EntryType::category` in the API; anything unlisted is 'other'.
const CATEGORY: Record<string, string> = {
  pr_authored: 'code', pr_merged: 'code', pr_development: 'code', revision_authored: 'code', development: 'code',
  bug_fixed: 'tickets', bug_filed: 'tickets', jira_completed: 'tickets', jira_story: 'tickets', jira_task: 'tickets', jira_epic: 'tickets', jira_progress: 'tickets',
  pr_reviewed: 'reviews', revision_reviewed: 'reviews', code_review: 'reviews',
  design_doc: 'docs', document: 'docs', confluence_page: 'docs', drive_created: 'docs', drive_edited: 'docs', drive_commented: 'docs',
  meeting: 'meetings', workshop: 'collaboration', mentoring: 'collaboration', presentation: 'collaboration', pairing: 'collaboration', cross_team: 'collaboration', interview: 'collaboration',
  learning: 'learning', onboarding: 'learning',
};
const CATEGORY_LABELS: [string, string][] = [['code', 'Code'], ['reviews', 'Reviews'], ['tickets', 'Tickets'], ['docs', 'Docs'], ['meetings', 'Meetings'], ['collaboration', 'Collaboration'], ['learning', 'Learning'], ['other', 'Other']];
const categoryOf = (entryType: string) => CATEGORY[entryType] ?? 'other';
const ENTRY_TYPES: EntryTypeDto[] = [
  ...['development', 'code_review', 'design_doc', 'document', 'meeting', 'workshop', 'mentoring', 'presentation', 'pairing', 'cross_team', 'interview', 'learning', 'onboarding', 'decision', 'process_improvement', 'unblocking', 'incident_response', 'investigation', 'other'].map((v) => ({ value: v, label: v.replace(/_/g, ' ').replace(/^./, (c) => c.toUpperCase()), group: categoryOf(v), manual: true })),
  ...['pr_authored', 'pr_merged', 'pr_reviewed', 'bug_fixed', 'bug_filed', 'revision_authored', 'revision_reviewed', 'jira_completed', 'jira_story', 'jira_task', 'jira_epic', 'jira_progress', 'confluence_page', 'drive_created', 'drive_edited', 'drive_commented'].map((v) => ({ value: v, label: v.replace(/_/g, ' ').replace(/^./, (c) => c.toUpperCase()), group: categoryOf(v), manual: false })),
];

/** Mirrors the API's `LogbookFilter` semantics (source is a comma list, category is derived from the type). */
function filterEntries(sp: URLSearchParams): DbEntry[] {
  const cycleId = num(sp.get('cycle_id')) ?? db.cycles.find((c) => c.is_active)?.id;
  const q = sp.get('q')?.toLowerCase();
  const pid = num(sp.get('priority_id'));
  const gid = num(sp.get('goal_id'));
  const goalPriorities = gid ? db.priorities.filter((p) => p.department_goal_id === gid).map((p) => p.id) : null;
  const sources = sp.get('source')?.split(',');
  return db.entries
    .filter((e) => e.cycle_id === cycleId)
    .filter((e) => !q || e.title.toLowerCase().includes(q) || (e.description ?? '').toLowerCase().includes(q))
    .filter((e) => !sources || sources.includes(e.source))
    .filter((e) => !sp.get('category') || categoryOf(e.entry_type) === sp.get('category'))
    .filter((e) => !sp.get('reach') || e.reach === sp.get('reach'))
    .filter((e) => !sp.get('complexity') || e.complexity === sp.get('complexity'))
    .filter((e) => !sp.get('role') || e.role === sp.get('role'))
    .filter((e) => !sp.get('team') || e.teams.includes(sp.get('team')!))
    .filter((e) => !sp.get('person') || e.collaborators.includes(sp.get('person')!))
    .filter((e) => !sp.get('from') || e.occurred_at >= sp.get('from')!)
    .filter((e) => !sp.get('to') || e.occurred_at <= sp.get('to')!)
    .filter((e) => pid === undefined || e.priority_id === pid)
    .filter((e) => !goalPriorities || (e.priority_id != null && goalPriorities.includes(e.priority_id)))
    .filter((e) => !sp.get('no_priority') || e.priority_id == null)
    .filter((e) => !sp.get('no_team') || e.teams.length === 0)
    .filter((e) => !sp.get('no_person') || e.collaborators.length === 0)
    .sort((a, b) => (a.occurred_at === b.occurred_at ? b.id - a.id : a.occurred_at < b.occurred_at ? 1 : -1));
}

const top = (xs: string[], n = 8) => {
  const m = new Map<string, number>();
  xs.forEach((x) => m.set(x, (m.get(x) ?? 0) + 1));
  return [...m].sort((a, b) => b[1] - a[1]).slice(0, n).map(([name, count]) => ({ name, count }));
};
const weekStart = (d: string) => format(startOfISOWeek(parseISO(d)), 'yyyy-MM-dd');

function stats(list: DbEntry[]): ReportStats {
  const perWeek = new Map<string, number>();
  list.forEach((e) => perWeek.set(weekStart(e.occurred_at), (perWeek.get(weekStart(e.occurred_at)) ?? 0) + 1));
  const byP = new Map<number | null, number>();
  list.forEach((e) => byP.set(e.priority_id, (byP.get(e.priority_id) ?? 0) + 1));
  return {
    entries: list.length,
    people: new Set(list.flatMap((e) => e.collaborators)).size,
    repos: new Set(list.map((e) => e.repository).filter(Boolean)).size,
    teams: new Set(list.flatMap((e) => e.teams)).size,
    per_week: [...perWeek].sort().map(([week_start, count]) => ({ week_start, count })),
    per_project: [...byP].map(([priority_id, count]) => ({ priority_id, name: db.priorities.find((p) => p.id === priority_id)?.title ?? 'No priority', count })),
    people_top: top(list.flatMap((e) => e.collaborators)),
    repos_top: top(list.map((e) => e.repository).filter((r): r is string => !!r)),
    teams_top: top(list.flatMap((e) => e.teams)),
  };
}

/** Priorities as the API returns them: entry count and latest update are computed on read. */
function priorityView(p: PriorityDto): PriorityDto {
  const ups = db.updates.filter((u) => u.priority_id === p.id).sort((a, b) => (a.created_at < b.created_at ? 1 : -1));
  return { ...p, entry_count: db.entries.filter((e) => e.priority_id === p.id).length, latest_update: ups[0] ?? null };
}

const cycleDto = ({ id, name, start_date, end_date, is_active }: CycleSummaryDto): CycleDto => ({ id, name, start_date, end_date, is_active });
const withCounts = (c: CycleSummaryDto): CycleSummaryDto => ({ ...c, entry_count: db.entries.filter((e) => e.cycle_id === c.id).length, priority_count: db.priorities.filter((p) => p.cycle_id === c.id).length });
const activeCycle = () => db.cycles.find((c) => c.is_active) ?? null;

const config = (): ConfigResponse => ({
  instance_name: 'Brag Frog',
  review_platform: 'Lattice',
  review_sections: db.sections,
  services: db.integrations.map((i) => ({ service: i.service, display_name: i.display_name, token_url: i.token_url, note: i.note, token_only: i.token_only, oauth_connect: i.oauth_connect, fields: i.fields })),
  clg_levels: [
    {
      id: 'senior', title: 'Senior Engineer (IC5)', one_liner: 'Own complex problems end to end.',
      problems: { task_size: 'Months', complexity: 'Ambiguous, cross-team problems.', risk_management: 'Plans rollouts.', domain_expertise: 'Deep in one area.', strategy: 'Shapes team plans.' },
      people: { influence: 'Team and adjacent teams', responsibility: 'Owns outcomes.', communication: 'Aligns stakeholders proactively.', change_management: '', team_contributions: 'Mentors.', dei: '' },
    },
    {
      id: 'staff', title: 'Staff Engineer (IC6)', one_liner: 'Set direction across teams.',
      problems: { task_size: 'Quarters', complexity: 'Org-wide ambiguity.', risk_management: '', domain_expertise: '', strategy: 'Sets direction.' },
      people: { influence: 'Organization', responsibility: '', communication: 'Shapes strategy.', change_management: '', team_contributions: '', dei: '' },
    },
  ],
});

// Meetings on Today are separate from the logbook rows so the logbook counts stay stable in tests.
const MEETINGS: [number, number, string, string][] = [[7000, 0, 'Platform sync', '10:00'], [7002, 1, '1:1 with Sam', '11:00'], [7003, 1, 'Design review', '14:00'], [7004, 2, 'Standup', '09:30'], [7005, 2, 'Quarterly planning', '15:00'], [7006, 3, 'Eng leads', '13:00'], [7007, 4, 'Demo', '10:00']];
function meetingEntry(id: number): DbEntry | undefined {
  const m = MEETINGS.find((x) => x[0] === id);
  if (!m) return undefined;
  const now = new Date().toISOString();
  return {
    id, week_id: db.week.id, cycle_id: 2, source: 'google_calendar', source_id: `cal-${id}`, source_url: null, title: m[2], description: null, entry_type: 'meeting', status: null, repository: null,
    occurred_at: format(addDays(parseISO(db.week.start_date), m[1]), 'yyyy-MM-dd'), teams: [], collaborators: [], outcome_statement: null, role: null, reach: null, complexity: null, priority_id: null,
    meeting_role: null, recurring_group: null, start_time: m[3], end_time: null, evidence_urls: null, impact_tags: null, decision: null, is_synced: true, created_at: now, updated_at: now,
  };
}

function todayPayload(): TodayResponse {
  const w = db.week;
  const todayStr = format(new Date(), 'yyyy-MM-dd');
  const days = Array.from({ length: 5 }, (_, i) => {
    const date = format(addDays(parseISO(w.start_date), i), 'yyyy-MM-dd');
    const meetings: MeetingDto[] = MEETINGS.filter((m) => m[1] === i).map((m) => ({ ...(meetingEntry(m[0]) as DbEntry), has_prep: !!db.preps[m[0]] })).map(({ cycle_id: _c, ...rest }) => (void _c, rest));
    return { date, label: format(parseISO(date), 'EEEE'), is_today: date === todayStr, meetings };
  });
  return {
    cycle: cycleDto(activeCycle()!),
    week: w,
    today: todayStr,
    days,
    active_work: db.entries.filter((e) => e.cycle_id === 2 && ['github', 'phabricator', 'jira', 'bugzilla'].includes(e.source)).slice(0, 8).map(dto),
    priorities: db.priorities
      .filter((p) => p.cycle_id === 2 && p.status === 'active')
      .map(priorityView)
      .map((p) => ({
        id: p.id, title: p.title, status: p.status, tracking_status: p.tracking_status, due_date: p.due_date, goal_title: db.goals.find((g) => g.id === p.department_goal_id)?.title ?? null,
        measure_type: p.measure_type, measure_start: p.measure_start, measure_target: p.measure_target, measure_current: p.measure_current, has_blocker: !!p.latest_update?.is_blocker,
        last_update_at: p.latest_update?.created_at ?? null,
      })),
    meeting_count: MEETINGS.length,
    focus_minutes: 660,
  };
}

function trends(range: TrendRange, cycleId: number): TrendsResponse {
  const list = db.entries.filter((e) => e.cycle_id === cycleId);
  const weeks = range === '4w' ? 4 : range === 'quarter' ? 13 : 16;
  const monday = startOfISOWeek(new Date());
  const weekly = Array.from({ length: weeks }, (_, i) => {
    const k = format(subWeeks(monday, weeks - 1 - i), 'yyyy-MM-dd');
    const inWeek = list.filter((e) => weekStart(e.occurred_at) === k);
    const n = (c: string) => inWeek.filter((e) => categoryOf(e.entry_type) === c).length;
    return { week_start: k, by_category: { code: n('code'), reviews: n('reviews'), tickets: n('tickets'), docs: n('docs'), meetings: n('meetings'), collaboration: n('collaboration'), learning: n('learning'), other: n('other') } };
  });
  const s = stats(list);
  const cat = new Map<string, number>();
  list.forEach((e) => cat.set(categoryOf(e.entry_type), (cat.get(categoryOf(e.entry_type)) ?? 0) + 1));
  const count = (k: keyof EntryDto, v: string) => list.filter((e) => e[k] === v).length;
  return {
    cycle_id: cycleId, range, from: weekly[0].week_start, to: format(new Date(), 'yyyy-MM-dd'),
    kpis: {
      docs_created: list.filter((e) => ['drive_created', 'confluence_page', 'document', 'design_doc'].includes(e.entry_type)).length,
      prs_opened: count('entry_type', 'pr_authored'), prs_reviewed: count('entry_type', 'pr_reviewed'), prs_merged: count('entry_type', 'pr_merged'),
      meetings: count('entry_type', 'meeting'),
      tickets_opened: list.filter((e) => CATEGORY[e.entry_type] === 'tickets' && e.entry_type !== 'jira_progress').length,
      tickets_closed: list.filter((e) => e.entry_type === 'jira_completed' || e.entry_type === 'bug_fixed').length,
    },
    weekly,
    categories: [...cat].map(([key, c]) => ({ key, name: CATEGORY_LABELS.find(([k]) => k === key)![1], count: c, types: [] })),
    priority_share: s.per_project.sort((a, b) => b.count - a.count).map((p) => ({ priority_id: p.priority_id, title: p.name, color: null, status: null, department_goal_id: null, department_goal_title: null, department_goal_status: null, count: p.count })),
    impact: { reach: [], complexity: [], role: [] },
    people: s.people_top, repos: s.repos_top, teams: s.teams_top,
    weekday: ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'].map((name, d) => ({ name, count: list.filter((e) => (parseISO(e.occurred_at).getDay() + 6) % 7 === d).length })),
  };
}

const prepResponse = (id: number): PrepResponse | null => {
  const e = meetingEntry(id);
  if (!e) return null;
  const note = db.preps[id];
  return { entry: dto(e), note: note ?? null };
};

export const handlers = [
  http.get(`${BASE}/me`, () =>
    HttpResponse.json({ user: db.profile, active_cycle: activeCycle() && cycleDto(activeCycle()!), has_ai: db.hasAi, public_only: false }),
  ),
  http.get(`${BASE}/public-config`, () => HttpResponse.json({ instance_name: 'Brag Frog', allowed_domain: 'example.com', public_only: false })),
  http.post(`${BASE}/auth/logout`, noContent),
  http.get(`${BASE}/config`, () => HttpResponse.json(config())),
  http.get(`${BASE}/meta/entry-types`, () => HttpResponse.json(ENTRY_TYPES)),

  // Cycles
  http.get(`${BASE}/cycles`, () => HttpResponse.json(db.cycles.map(withCounts))),
  http.post(`${BASE}/cycles`, async ({ request }) => {
    const b = (await request.json()) as { name: string; start_date: string; end_date: string };
    if (!b.name?.trim()) return err(422, 'invalid_input', 'Name is required');
    if (b.end_date < b.start_date) return err(422, 'invalid_input', 'end_date must not be before start_date');
    const c: CycleSummaryDto = { id: db.nextId++, name: b.name, start_date: b.start_date, end_date: b.end_date, is_active: false, entry_count: 0, priority_count: 0 };
    db.cycles.unshift(c);
    return HttpResponse.json(cycleDto(c), { status: 201 });
  }),
  http.post(`${BASE}/cycles/:id/activate`, ({ params }) => {
    const target = db.cycles.find((c) => c.id === Number(params.id));
    if (!target) return err(404, 'not_found', 'Cycle not found');
    db.cycles.forEach((c) => (c.is_active = c.id === target.id));
    return HttpResponse.json(cycleDto(target));
  }),
  http.delete(`${BASE}/cycles/:id`, async ({ params, request }) => {
    const c = db.cycles.find((x) => x.id === Number(params.id));
    const b = (await request.json()) as { confirm_name: string };
    if (!c) return err(404, 'not_found', 'Cycle not found');
    if (b.confirm_name !== c.name) return err(422, 'invalid_input', 'confirm_name does not match the cycle name');
    db.cycles.splice(db.cycles.indexOf(c), 1);
    return noContent();
  }),
  http.get(`${BASE}/cycles/:id/bundle`, ({ params }) => {
    const id = Number(params.id);
    const c = db.cycles.find((x) => x.id === id);
    if (!c) return err(404, 'not_found', 'Cycle not found');
    const weeks = new Map<string, DbEntry[]>();
    db.entries.filter((e) => e.cycle_id === id).forEach((e) => weeks.set(weekStart(e.occurred_at), [...(weeks.get(weekStart(e.occurred_at)) ?? []), e]));
    const ps = db.priorities.filter((p) => p.cycle_id === id).map(priorityView);
    return HttpResponse.json({
      cycle: cycleDto(c),
      exported_at: new Date().toISOString(),
      review: { cycle_id: id, sections: db.sections.map((s) => sectionFor(id, s)), entry_count: db.entries.filter((e) => e.cycle_id === id).length },
      priorities: {
        cycle_id: id,
        goals: db.goals.filter((g) => g.cycle_id === id).map((goal) => ({ goal, priorities: ps.filter((p) => p.department_goal_id === goal.id) })),
        unassigned: ps.filter((p) => !db.goals.some((g) => g.id === p.department_goal_id)),
        counts: { total: ps.length, active: ps.filter((p) => p.status === 'active').length, not_started: 0, on_hold: 0, completed: ps.filter((p) => p.status === 'completed').length, cancelled: 0 },
      },
      timelines: ps.map((p) => ({ priority_id: p.id, updates: db.updates.filter((u) => u.priority_id === p.id) })),
      weeks: [...weeks].sort().map(([start, entries]) => ({
        iso_week: Number(format(parseISO(start), 'I')), year: Number(format(parseISO(start), 'R')), start_date: start, end_date: format(addDays(parseISO(start), 6), 'yyyy-MM-dd'), entries: entries.map(dto),
      })),
    });
  }),

  // Entries: static paths before :id
  http.get(`${BASE}/entries/facets`, ({ request }) => {
    const cycleId = num(new URL(request.url).searchParams.get('cycle_id')) ?? 2;
    const es = db.entries.filter((e) => e.cycle_id === cycleId);
    const sources = new Map<string, number>();
    es.forEach((e) => sources.set(e.source, (sources.get(e.source) ?? 0) + 1));
    return HttpResponse.json({
      teams: [...new Set(es.flatMap((e) => e.teams))].sort(),
      people: [...new Set(es.flatMap((e) => e.collaborators))].sort(),
      sources: [...sources].map(([source, count]) => ({ source, count })),
      categories: CATEGORY_LABELS.map(([value, label]) => ({ value, label, count: es.filter((e) => categoryOf(e.entry_type) === value).length })).filter((c) => c.count > 0),
      total: es.length,
    });
  }),
  http.get(`${BASE}/entries`, async ({ request }) => {
    const sp = new URL(request.url).searchParams;
    const all = filterEntries(sp);
    const limit = num(sp.get('limit')) ?? 50;
    const offset = Number(sp.get('cursor') || 0);
    await delay(20);
    return HttpResponse.json({ items: all.slice(offset, offset + limit).map(dto), next_cursor: offset + limit < all.length ? String(offset + limit) : null, total_matching: all.length });
  }),
  http.post(`${BASE}/entries/bulk`, async ({ request }) => {
    const { ids, merge_mode, teams, collaborators, ...patch } = (await request.json()) as BulkRequest;
    const merge = (a: string[], b?: string[]) => (b ? (merge_mode === 'replace' ? b : [...new Set([...a, ...b])]) : a);
    const hit = db.entries.filter((e) => ids.includes(e.id));
    hit.forEach((e) => Object.assign(e, patch, { teams: merge(e.teams, teams), collaborators: merge(e.collaborators, collaborators) }));
    return HttpResponse.json({ updated: hit.length });
  }),
  http.post(`${BASE}/entries`, async ({ request }) => {
    const b = (await request.json()) as EntryInput;
    if (!b.title?.trim()) return err(422, 'invalid_input', 'Title is required');
    const now = new Date().toISOString();
    const e: DbEntry = {
      id: db.nextId++, week_id: 1, cycle_id: activeCycle()!.id, source: 'manual', source_id: null, source_url: b.source_url ?? null, title: b.title, description: b.description ?? null, entry_type: b.entry_type,
      status: null, repository: null, occurred_at: b.occurred_at ?? format(new Date(), 'yyyy-MM-dd'), teams: b.teams ?? [], collaborators: b.collaborators ?? [], outcome_statement: null, role: b.role ?? null,
      reach: b.reach ?? null, complexity: b.complexity ?? null, priority_id: b.priority_id ?? null, meeting_role: null, recurring_group: null, start_time: null, end_time: null, evidence_urls: null,
      impact_tags: null, decision: null, is_synced: false, created_at: now, updated_at: now,
    };
    db.entries.unshift(e);
    return HttpResponse.json(dto(e), { status: 201 });
  }),
  http.get(`${BASE}/entries/:id`, ({ params }) => {
    const e = db.entries.find((x) => x.id === Number(params.id));
    return e ? HttpResponse.json(dto(e)) : err(404, 'not_found', 'Entry not found');
  }),
  http.put(`${BASE}/entries/:id`, async ({ params, request }) => {
    const e = db.entries.find((x) => x.id === Number(params.id));
    if (!e) return err(404, 'not_found', 'Entry not found');
    const b = (await request.json()) as EntryInput;
    if (!b.title?.trim()) return err(422, 'invalid_input', 'Title is required');
    // PUT replaces the editable fields: absent ones are cleared.
    Object.assign(e, {
      title: b.title, entry_type: e.is_synced ? e.entry_type : b.entry_type, occurred_at: b.occurred_at ?? e.occurred_at, description: b.description ?? null, priority_id: b.priority_id ?? null,
      teams: b.teams ?? [], collaborators: b.collaborators ?? [], reach: b.reach ?? null, complexity: b.complexity ?? null, role: b.role ?? null, updated_at: new Date().toISOString(),
    });
    return HttpResponse.json(dto(e));
  }),
  http.delete(`${BASE}/entries/:id`, ({ params }) => {
    const i = db.entries.findIndex((x) => x.id === Number(params.id));
    if (i < 0) return err(404, 'not_found', 'Entry not found');
    db.entries.splice(i, 1);
    return noContent();
  }),
  http.post(`${BASE}/entries/:id/classify`, async ({ params, request }) => {
    const e = db.entries.find((x) => x.id === Number(params.id));
    if (!e) return err(404, 'not_found', 'Entry not found');
    const { meeting_role, teams } = (await request.json()) as { meeting_role: string; teams?: string[] };
    Object.assign(e, { meeting_role, ...(teams && { teams }) });
    return noContent();
  }),
  http.post(`${BASE}/entries/:id/exclude`, ({ params }) => {
    db.entries = db.entries.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),

  // Today and meeting prep
  http.get(`${BASE}/today`, () => HttpResponse.json(todayPayload())),
  http.get(`${BASE}/meetings/:id/prep`, ({ params }) => {
    const r = prepResponse(Number(params.id));
    return r ? HttpResponse.json(r) : err(404, 'not_found', 'Meeting not found');
  }),
  http.put(`${BASE}/meetings/:id/prep`, async ({ params, request }) => {
    const id = Number(params.id);
    if (!meetingEntry(id)) return err(404, 'not_found', 'Meeting not found');
    const b = (await request.json()) as SavePrepRequest;
    db.preps[id] = { notes: b.notes || null, meeting_goal: b.meeting_goal || null, doc_urls: b.doc_urls || null, updated_at: new Date().toISOString() };
    return HttpResponse.json(prepResponse(id));
  }),
  http.post(`${BASE}/meetings/:id/prep/draft`, async () => {
    await delay(50);
    return HttpResponse.json({ content: '- Confirm runner capacity ask\n- Share autoscaling results\n- Agree on next steps' });
  }),

  // Priorities and goals
  http.get(`${BASE}/priorities`, ({ request }) => {
    const cycleId = num(new URL(request.url).searchParams.get('cycle_id')) ?? 2;
    const ps = db.priorities.filter((p) => p.cycle_id === cycleId).map(priorityView);
    const n = (s: string) => ps.filter((p) => p.status === s).length;
    return HttpResponse.json({
      cycle_id: cycleId,
      goals: db.goals.filter((g) => g.cycle_id === cycleId).map((goal) => ({ goal, priorities: ps.filter((p) => p.department_goal_id === goal.id) })),
      unassigned: ps.filter((p) => p.department_goal_id == null || !db.goals.some((g) => g.id === p.department_goal_id)),
      counts: { total: ps.length, active: n('active'), not_started: n('not_started'), on_hold: n('on_hold'), completed: n('completed'), cancelled: n('cancelled') },
    });
  }),
  http.post(`${BASE}/priorities/import`, () => HttpResponse.json({ created: 2, updated: 1, skipped: 1, errors: ['Row 5: missing title'] })),
  http.post(`${BASE}/priorities`, async ({ request }) => {
    const b = (await request.json()) as CreatePriorityRequest;
    if (!b.title?.trim()) return err(422, 'invalid_input', 'Title is required');
    const p: PriorityDto = {
      id: db.nextId++, cycle_id: b.cycle_id ?? 2, title: b.title, status: b.status ?? 'active', color: null, sort_order: 99, scope: b.scope ?? null, started_at: null, completed_at: null,
      impact_narrative: b.impact_narrative ?? null, department_goal_id: b.department_goal_id ?? null, created_at: new Date().toISOString(), priority_level: b.priority_level ?? null,
      measure_type: b.measure_type ?? null, measure_start: b.measure_start ?? null, measure_target: b.measure_target ?? null, measure_current: null, description: b.description ?? null,
      tracking_status: b.tracking_status ?? 'on_track', due_date: b.due_date ?? null, tier: b.tier ?? null, entry_count: 0, latest_update: null,
    };
    db.priorities.push(p);
    return HttpResponse.json(priorityView(p), { status: 201 });
  }),
  http.put(`${BASE}/priorities/:id`, async ({ params, request }) => {
    const p = db.priorities.find((x) => x.id === Number(params.id));
    if (!p) return err(404, 'not_found', 'Priority not found');
    const b = (await request.json()) as UpdatePriorityRequest;
    Object.assign(p, Object.fromEntries(Object.entries(b).filter(([, v]) => v !== undefined)));
    return HttpResponse.json(priorityView(p));
  }),
  http.delete(`${BASE}/priorities/:id`, ({ params }) => {
    db.priorities = db.priorities.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),
  http.get(`${BASE}/priorities/:id/updates`, ({ params }) => HttpResponse.json(db.updates.filter((u) => u.priority_id === Number(params.id)).sort((a, b) => (a.created_at < b.created_at ? 1 : -1)))),
  http.post(`${BASE}/priorities/:id/updates`, async ({ params, request }) => {
    const b = (await request.json()) as PostUpdateRequest;
    if (!b.comment?.trim()) return err(422, 'invalid_input', 'Say what changed');
    const u = { id: db.nextId++, priority_id: Number(params.id), tracking_status: b.tracking_status ?? null, measure_value: b.measure_value ?? null, comment: b.comment, is_blocker: !!b.is_blocker, tradeoff_text: null, created_at: new Date().toISOString() };
    db.updates.push(u);
    const p = db.priorities.find((x) => x.id === Number(params.id));
    if (p) {
      if (b.tracking_status) p.tracking_status = b.tracking_status;
      if (b.measure_value != null) p.measure_current = b.measure_value;
    }
    return HttpResponse.json(u, { status: 201 });
  }),
  http.post(`${BASE}/goals`, async ({ request }) => {
    const b = (await request.json()) as { title?: string; cycle_id?: number };
    if (!b.title?.trim()) return err(422, 'invalid_input', 'Title is required');
    const g = { id: db.nextId++, cycle_id: b.cycle_id ?? 2, title: b.title, description: null, status: 'active', sort_order: 99, source: 'manual', created_at: new Date().toISOString() };
    db.goals.push(g);
    return HttpResponse.json(g, { status: 201 });
  }),
  http.put(`${BASE}/goals/:id`, async ({ params, request }) => {
    const g = db.goals.find((x) => x.id === Number(params.id));
    if (!g) return err(404, 'not_found', 'Goal not found');
    const b = (await request.json()) as { title?: string; status?: string };
    Object.assign(g, b);
    if (b.status === 'completed') db.priorities.filter((p) => p.department_goal_id === g.id && p.status === 'active').forEach((p) => Object.assign(p, { status: 'completed', tracking_status: 'complete' }));
    return HttpResponse.json(g);
  }),
  http.delete(`${BASE}/goals/:id`, ({ params }) => {
    db.goals = db.goals.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),

  // Review
  http.get(`${BASE}/cycles/:id/review`, ({ params }) =>
    HttpResponse.json({ cycle_id: Number(params.id), sections: db.sections.map((s) => sectionFor(Number(params.id), s)), entry_count: db.entries.filter((e) => e.cycle_id === Number(params.id)).length }),
  ),
  http.put(`${BASE}/cycles/:id/review/:slug`, async ({ params, request }) => {
    const b = (await request.json()) as { content: string; content_format: string };
    db.reviews[`${params.id}:${params.slug}`] = { ...b, generated_at: null };
    const cfg = db.sections.find((s) => s.slug === params.slug);
    if (!cfg) return err(404, 'not_found', 'Unknown section');
    return HttpResponse.json(sectionFor(Number(params.id), cfg));
  }),
  http.post(`${BASE}/cycles/:id/review/:slug/draft`, async () => {
    await delay(60);
    return HttpResponse.json({ markdown: '## 1. Drafted example\n\nA first draft from your logbook and priorities.' });
  }),
  http.get(`${BASE}/cycles/:id/archive`, ({ params }) => HttpResponse.json({ ...db.archive, cycle_id: Number(params.id) })),

  // Reports
  http.get(`${BASE}/reports`, ({ request }) => {
    const cycleId = num(new URL(request.url).searchParams.get('cycle_id')) ?? 2;
    return HttpResponse.json(db.reports.filter((r) => r.cycle_id === cycleId).map(({ id, cycle_id, title, filters, entry_count, generated_at }) => ({ id, cycle_id, title, filters, entry_count, generated_at })).reverse());
  }),
  http.post(`${BASE}/reports`, async ({ request }) => {
    const b = (await request.json()) as CreateReportRequest;
    const sp = new URLSearchParams(Object.entries({ ...b.filters, cycle_id: b.cycle_id }).filter(([, v]) => v != null && v !== '').map(([k, v]) => [k, String(v)]));
    const list = filterEntries(sp);
    if (list.length === 0) return err(422, 'empty_selection', 'No entries match these filters');
    await delay(30);
    const key: ReportKeyEntry[] = list.slice(0, 10).map((e) => ({ id: e.id, title: e.title, entry_type: e.entry_type, source: e.source, source_url: e.source_url, occurred_at: e.occurred_at, priority_id: e.priority_id, repository: e.repository, reach: e.reach, complexity: e.complexity }));
    const r = {
      id: db.nextId++, cycle_id: b.cycle_id, title: b.title || 'All projects · report', filters: b.filters, entry_count: list.length, generated_at: new Date().toISOString(), updated_at: new Date().toISOString(),
      model_used: b.with_ai ? 'mock' : null, stats: stats(list), narrative: { content: b.with_ai ? 'Focused on CI reliability and platform work with several collaborators.' : '', content_format: 'markdown' },
      key_entries: key, ai_error: null,
    };
    db.reports.push(r);
    return HttpResponse.json(r, { status: 201 });
  }),
  http.get(`${BASE}/reports/:id`, ({ params }) => {
    const r = db.reports.find((x) => x.id === Number(params.id));
    return r ? HttpResponse.json(r) : err(404, 'not_found', 'Report not found');
  }),
  http.delete(`${BASE}/reports/:id`, ({ params }) => {
    db.reports = db.reports.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),

  // Trends
  http.get(`${BASE}/trends`, ({ request }) => {
    const sp = new URL(request.url).searchParams;
    return HttpResponse.json(trends((sp.get('range') as TrendRange) ?? 'cycle', num(sp.get('cycle_id')) ?? 2));
  }),

  // Integrations and sync
  http.get(`${BASE}/integrations`, () => HttpResponse.json({ services: db.integrations })),
  http.put(`${BASE}/integrations/:service`, async ({ params, request }) => {
    const i = db.integrations.find((x) => x.service === params.service);
    if (!i) return err(404, 'not_found', 'Unknown service');
    const b = (await request.json()) as Partial<IntegrationInput>;
    if (b.token === 'bad') return err(422, 'invalid_input', 'Connection failed: Token rejected');
    i.fields = i.fields.map((f) => (f.name in b && b[f.name as keyof IntegrationInput] != null ? { ...f, value: b[f.name as keyof IntegrationInput] as string | boolean } : f));
    Object.assign(i, { is_enabled: true, has_token: i.has_token || !!b.token || i.oauth_connect, last_sync_error: null, last_sync_status: i.last_sync_status === 'error' ? 'success' : i.last_sync_status });
    return HttpResponse.json(i);
  }),
  http.post(`${BASE}/integrations/:service/test`, async ({ params }) => {
    const i = db.integrations.find((x) => x.service === params.service);
    return HttpResponse.json(i?.last_sync_status === 'error' ? { ok: false, message: 'Token rejected' } : { ok: true, message: 'Connection works' });
  }),
  http.delete(`${BASE}/integrations/:service/excluded/:id`, ({ params }) => {
    const i = db.integrations.find((x) => x.service === params.service);
    if (i) {
      i.excluded_events = i.excluded_events.filter((x) => x.id !== params.id);
      i.excluded_files = i.excluded_files.filter((x) => x.id !== params.id);
    }
    return noContent();
  }),
  http.delete(`${BASE}/integrations/:service`, ({ params }) => {
    const i = db.integrations.find((x) => x.service === params.service);
    if (i) Object.assign(i, { is_enabled: false, has_token: false, last_sync_at: null, last_sync_status: null, last_sync_error: null });
    return noContent();
  }),
  http.post(`${BASE}/sync`, () => {
    db.sync = { running: true, since: Date.now() };
    return HttpResponse.json({ started: true }, { status: 202 });
  }),
  http.post(`${BASE}/sync/:service`, () => {
    db.sync = { running: true, since: Date.now() };
    return HttpResponse.json({ started: true }, { status: 202 });
  }),
  http.get(`${BASE}/sync/status`, () => {
    // A mock sync "finishes" after 3 s so the polling behaviour is visible.
    if (db.sync.running && Date.now() - db.sync.since > 3000) db.sync.running = false;
    const failing = db.integrations.find((i) => i.last_sync_status === 'error');
    const body: SyncStatus = {
      running: db.sync.running, current_service: db.sync.running ? 'github' : null, services_remaining: db.sync.running ? 2 : 0, services_total: db.integrations.length,
      last_synced_at: new Date().toISOString(), last_error: failing ? `${failing.display_name}: ${failing.last_sync_error}` : null,
      services: db.integrations.map((i) => ({ service: i.service, display_name: i.display_name, is_enabled: i.is_enabled, last_sync_at: i.last_sync_at, last_sync_status: i.last_sync_status, last_sync_error: i.last_sync_error })),
      recent_logs: [{ id: 1, service: 'github', started_at: new Date().toISOString(), completed_at: new Date().toISOString(), status: 'success', entries_created: 12, entries_updated: 3, entries_deleted: 0, entries_fetched: 15, entries_skipped: 0, error_message: null }],
    };
    return HttpResponse.json(body);
  }),

  // Settings
  http.get(`${BASE}/settings/profile`, () => HttpResponse.json(db.profile)),
  http.put(`${BASE}/settings/profile`, async ({ request }) => {
    const b = (await request.json()) as UpdateProfileRequest;
    const clear = (v: string | undefined) => (v ? v : null);
    Object.assign(db.profile, {
      role: clear(b.role), wants_promotion: b.wants_promotion, display_name: clear(b.display_name), team: clear(b.team), manager_name: clear(b.manager_name), skip_level_name: clear(b.skip_level_name),
      direct_reports: clear(b.direct_reports), timezone: clear(b.timezone), week_start: clear(b.week_start), work_start_time: clear(b.work_start_time), work_end_time: clear(b.work_end_time),
    });
    return HttpResponse.json(db.profile);
  }),
  http.get(`${BASE}/settings/people`, () => HttpResponse.json(db.people)),
  http.post(`${BASE}/settings/people`, async ({ request }) => {
    const b = (await request.json()) as { email: string; display_name: string; team?: string };
    if (!b.email?.includes('@')) return err(422, 'invalid_input', 'Enter a valid email');
    const existing = db.people.find((p) => p.email === b.email);
    if (existing) Object.assign(existing, { display_name: b.display_name, team: b.team ?? null });
    else db.people.push({ id: db.nextId++, email: b.email, display_name: b.display_name, team: b.team ?? null });
    return HttpResponse.json(db.people);
  }),
  http.delete(`${BASE}/settings/people/:id`, ({ params }) => {
    db.people = db.people.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),
];
