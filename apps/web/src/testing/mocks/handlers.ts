import { http, HttpResponse, delay } from 'msw';
import { addDays, format, parseISO, startOfISOWeek, subWeeks } from 'date-fns';
import type { AppConfig, BulkPatch, Cycle, Entry, EntrySummary, PriorityCard, ReportInput, ReportStats, Today, Trends, TrendRange } from '@/types/api/models';
import { db, sectionFor, type DbEntry } from './db';

const BASE = '/api/v1';
const err = (status: number, code: string, message: string) => HttpResponse.json({ error: { code, message } }, { status });
const noContent = () => new HttpResponse(null, { status: 204 });
const summary = (e: Entry): EntrySummary => ({
  id: e.id, source: e.source, source_url: e.source_url, title: e.title, entry_type: e.entry_type, status: e.status, repository: e.repository, occurred_at: e.occurred_at,
  teams: e.teams, collaborators: e.collaborators, priority_id: e.priority_id, reach: e.reach, complexity: e.complexity, role: e.role, meeting_role: e.meeting_role,
});
const num = (v: string | null) => (v ? Number(v) : undefined);

function filterEntries(sp: URLSearchParams): DbEntry[] {
  const cycleId = num(sp.get('cycle_id')) ?? db.cycles.find((c) => c.is_active)?.id;
  const q = sp.get('q')?.toLowerCase();
  const pid = num(sp.get('priority_id'));
  const gid = num(sp.get('goal_id'));
  const goalPriorities = gid ? db.priorities.filter((p) => p.department_goal_id === gid).map((p) => p.id) : null;
  return db.entries
    .filter((e) => e.cycle_id === cycleId)
    .filter((e) => !q || e.title.toLowerCase().includes(q) || e.description.toLowerCase().includes(q))
    .filter((e) => !sp.get('source') || e.source === sp.get('source'))
    .filter((e) => !sp.get('type') || e.entry_type === sp.get('type'))
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
    .filter((e) => !sp.get('needs_classify') || (e.entry_type === 'meeting' && !e.meeting_role))
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

function priorityCard(p: (typeof db.priorities)[number]): PriorityCard {
  const ups = db.updates.filter((u) => u.priority_id === p.id).sort((a, b) => (a.created_at < b.created_at ? 1 : -1));
  return { ...p, entry_count: db.entries.filter((e) => e.priority_id === p.id).length, latest_update: ups[0] ?? null, blocker_count: ups.filter((u) => u.is_blocker).length };
}

const activeCycle = () => db.cycles.find((c) => c.is_active) ?? null;
const withCounts = (c: Cycle): Cycle => ({ ...c, entry_count: db.entries.filter((e) => e.cycle_id === c.id).length, priority_count: db.priorities.filter((p) => p.cycle_id === c.id).length });

const svc = (service: string, label: string, extra: Partial<AppConfig['services'][number]> = {}) => ({
  service, label, token_url: null, note: null, base_url_placeholder: null, email_placeholder: null, org_placeholder: null, default_orgs: null, default_base_url: null, ...extra,
});
const config = (): AppConfig => ({
  instance_name: 'Brag Frog',
  atlassian_base_url: 'https://example.atlassian.net',
  review_sections: db.sections,
  services: [
    svc('github', 'GitHub', { token_url: 'https://github.com/settings/tokens/new', note: 'Create a classic token.', org_placeholder: 'my-org' }),
    svc('phabricator', 'Phabricator', { base_url_placeholder: 'https://phabricator.example.com', default_base_url: 'https://phabricator.example.com' }),
    svc('bugzilla', 'Bugzilla', { note: 'API key is optional.', base_url_placeholder: 'https://bugzilla.example.com', email_placeholder: 'you@example.com' }),
    svc('atlassian', 'Atlassian', { token_url: 'https://id.atlassian.com/manage-profile/security/api-tokens', note: 'Syncs Jira and Confluence.', base_url_placeholder: 'https://your-org.atlassian.net', email_placeholder: 'you@example.com' }),
    svc('google_calendar', 'Google Calendar'),
    svc('google_drive', 'Google Drive'),
  ],
  clg_levels: [
    { id: 'senior', title: 'Senior Engineer (IC5)', one_liner: 'Own complex problems end to end.', problems: { task_size: 'Months', complexity: 'Ambiguous, cross-team problems.' }, people: { influence: 'Team and adjacent teams', communication: 'Aligns stakeholders proactively.' } },
    { id: 'staff', title: 'Staff Engineer (IC6)', one_liner: 'Set direction across teams.', problems: { task_size: 'Quarters', complexity: 'Org-wide ambiguity.' }, people: { influence: 'Organization', communication: 'Shapes strategy.' } },
  ],
});

function todayPayload(): Today {
  const w = db.week;
  const start = parseISO(w.week_start);
  const todayStr = format(new Date(), 'yyyy-MM-dd');
  const fixed = [
    [{ entry_id: 7000, title: 'Platform sync', start_time: '10:00', end_time: '10:30' }],
    [{ entry_id: 7002, title: '1:1 with Sam', start_time: '11:00', end_time: '11:30' }, { entry_id: 7003, title: 'Design review', start_time: '14:00', end_time: '15:00' }],
    [{ entry_id: 7004, title: 'Standup', start_time: '09:30', end_time: '09:45' }, { entry_id: 7005, title: 'Quarterly planning', start_time: '15:00', end_time: '16:00' }],
    [{ entry_id: 7006, title: 'Eng leads', start_time: '13:00', end_time: '14:00' }],
    [{ entry_id: 7007, title: 'Demo', start_time: '10:00', end_time: '11:00' }],
  ];
  const days = fixed.map((ms, i) => {
    const d = format(addDays(start, i), 'yyyy-MM-dd');
    return { date: d, label: format(addDays(start, i), 'EEE') + (d === todayStr ? ' · today' : ''), is_today: d === todayStr, meetings: ms.map((m) => ({ ...m, recurring_group: null })) };
  });
  return {
    week: w,
    meetings_by_day: days,
    prep_map: { '7002': true, '7006': true, ...Object.fromEntries(Object.keys(db.preps).map((k) => [k, true])) },
    active_work: db.entries.filter((e) => e.cycle_id === 2 && ['github', 'phabricator', 'jira', 'bugzilla'].includes(e.source)).slice(0, 8).map(summary),
    priorities: db.priorities.filter((p) => p.cycle_id === 2 && p.status === 'active').map(priorityCard),
    focus_days: days.map((d) => ({ date: d.date, focus_hours: 2.5, meeting_hours: d.meetings.length })),
    counts: { meetings: days.reduce((n, d) => n + d.meetings.length, 0), focus_hours: 11 },
  };
}

function trends(range: TrendRange, cycleId?: number): Trends {
  const list = db.entries.filter((e) => e.cycle_id === (cycleId ?? 2));
  const weeks = range === '4w' ? 4 : range === 'quarter' ? 13 : 16;
  const monday = startOfISOWeek(new Date());
  const kind = (e: Entry) => (/review/.test(e.entry_type) ? 'review' : e.entry_type === 'meeting' ? 'meeting' : ['github', 'phabricator', 'bugzilla', 'jira'].includes(e.source) ? 'code' : 'other');
  const weekly = Array.from({ length: weeks }, (_, i) => {
    const k = format(subWeeks(monday, weeks - 1 - i), 'yyyy-MM-dd');
    const inWeek = list.filter((e) => weekStart(e.occurred_at) === k);
    const n = (t: string) => inWeek.filter((e) => kind(e) === t).length;
    return { week_start: k, by_type: { code: n('code'), review: n('review'), meeting: n('meeting'), other: n('other') } };
  });
  const s = stats(list);
  return {
    kpis: { entries: list.length, entries_delta_pct: 8, people: s.people, repos: s.repos, priority_linked_pct: Math.round((list.filter((e) => e.priority_id != null).length / Math.max(1, list.length)) * 100) },
    weekly,
    priority_share: s.per_project.sort((a, b) => b.count - a.count),
    people: s.people_top,
    repos: s.repos_top,
    teams: s.teams_top,
    weekday_hour: null,
  };
}

export const handlers = [
  http.get(`${BASE}/me`, () =>
    HttpResponse.json({ user: { id: 1, email: 'bgruber@example.com', name: 'Bastian Gruber', display_name: 'Bastian', avatar_url: null, role: 'senior' }, active_cycle: activeCycle() && withCounts(activeCycle()!), has_ai: db.hasAi, public_only: false }),
  ),
  http.get(`${BASE}/public-config`, () => HttpResponse.json({ instance_name: 'Brag Frog', allowed_domain_hint: '@example.com', public_only: false })),
  http.post(`${BASE}/auth/logout`, noContent),
  http.get(`${BASE}/config`, () => HttpResponse.json(config())),

  // Cycles
  http.get(`${BASE}/cycles`, () => HttpResponse.json(db.cycles.map(withCounts))),
  http.post(`${BASE}/cycles`, async ({ request }) => {
    const b = (await request.json()) as { name: string; start_date: string; end_date: string; activate: boolean };
    if (!b.name) return err(422, 'invalid_input', 'Name is required');
    if (b.activate) db.cycles.forEach((c) => (c.is_active = false));
    const c: Cycle = { id: db.nextId++, name: b.name, start_date: b.start_date, end_date: b.end_date, is_active: b.activate, entry_count: 0, priority_count: 0 };
    db.cycles.unshift(c);
    return HttpResponse.json(c, { status: 201 });
  }),
  http.post(`${BASE}/cycles/:id/activate`, ({ params }) => {
    db.cycles.forEach((c) => (c.is_active = c.id === Number(params.id)));
    return noContent();
  }),
  http.delete(`${BASE}/cycles/:id`, async ({ params, request }) => {
    const c = db.cycles.find((x) => x.id === Number(params.id));
    const b = (await request.json()) as { confirm_name: string };
    if (!c) return err(404, 'not_found', 'Cycle not found');
    if (b.confirm_name !== c.name) return err(422, 'invalid_input', 'Name does not match');
    db.cycles.splice(db.cycles.indexOf(c), 1);
    return noContent();
  }),
  http.get(`${BASE}/cycles/:id/bundle`, ({ params }) => {
    const id = Number(params.id);
    const c = db.cycles.find((x) => x.id === id);
    if (!c) return err(404, 'not_found', 'Cycle not found');
    const weeks = new Map<string, Entry[]>();
    db.entries.filter((e) => e.cycle_id === id).forEach((e) => weeks.set(weekStart(e.occurred_at), [...(weeks.get(weekStart(e.occurred_at)) ?? []), e]));
    return HttpResponse.json({
      cycle: withCounts(c),
      review: db.sections.map((s) => { const r = sectionFor(id, s); return { slug: s.slug, title: s.title, content: r.content, content_format: r.content_format }; }).filter((r) => r.content),
      goals: db.goals.filter((g) => g.cycle_id === id),
      priorities: db.priorities.filter((p) => p.cycle_id === id).map((p) => ({ ...p, updates: db.updates.filter((u) => u.priority_id === p.id) })),
      weeks: [...weeks].sort().map(([week_start, entries], i) => ({ week_start, iso_week: i + 1, entries })),
    });
  }),

  // Entries: static paths before :id
  http.get(`${BASE}/entries/facets`, ({ request }) => {
    const cycleId = num(new URL(request.url).searchParams.get('cycle_id')) ?? 2;
    const es = db.entries.filter((e) => e.cycle_id === cycleId);
    return HttpResponse.json({
      teams: [...new Set(es.flatMap((e) => e.teams))].sort(),
      people: [...new Set(es.flatMap((e) => e.collaborators))].sort(),
      sources: [...new Set(es.map((e) => e.source))],
      counts: { total: es.length, no_priority: es.filter((e) => e.priority_id == null).length, no_team: es.filter((e) => e.teams.length === 0).length, needs_classify: es.filter((e) => e.entry_type === 'meeting' && !e.meeting_role).length },
    });
  }),
  http.get(`${BASE}/entries`, async ({ request }) => {
    const sp = new URL(request.url).searchParams;
    const all = filterEntries(sp);
    const limit = num(sp.get('limit')) ?? 50;
    const offset = Number(sp.get('cursor') || 0);
    await delay(20);
    return HttpResponse.json({ items: all.slice(offset, offset + limit).map(summary), next_cursor: offset + limit < all.length ? String(offset + limit) : null, total_matching: all.length });
  }),
  http.post(`${BASE}/entries/bulk`, async ({ request }) => {
    const { ids, patch } = (await request.json()) as { ids: number[]; patch: BulkPatch };
    db.entries.filter((e) => ids.includes(e.id)).forEach((e) => Object.assign(e, patch));
    return noContent();
  }),
  http.post(`${BASE}/entries`, async ({ request }) => {
    const b = (await request.json()) as Partial<Entry> & { cycle_id?: number };
    if (!b.title?.trim()) return err(422, 'invalid_input', 'Title is required');
    const now = new Date().toISOString();
    const e: DbEntry = {
      id: db.nextId++, week_id: 1, cycle_id: b.cycle_id ?? 2, source: 'manual', source_id: null, source_url: null, title: b.title, description: b.description ?? '', entry_type: b.entry_type ?? 'other', status: null,
      repository: null, occurred_at: b.occurred_at ?? format(new Date(), 'yyyy-MM-dd'), teams: b.teams ?? [], collaborators: b.collaborators ?? [], outcome_statement: b.outcome_statement ?? null, role: b.role ?? null,
      reach: b.reach ?? null, complexity: b.complexity ?? null, priority_id: b.priority_id ?? null, meeting_role: null, recurring_group: null, start_time: null, end_time: null, evidence_urls: [], impact_tags: [], decision: null,
      created_at: now, updated_at: now,
    };
    db.entries.unshift(e);
    return HttpResponse.json(e, { status: 201 });
  }),
  http.get(`${BASE}/entries/:id`, ({ params }) => {
    const e = db.entries.find((x) => x.id === Number(params.id));
    return e ? HttpResponse.json(e) : err(404, 'not_found', 'Entry not found');
  }),
  http.put(`${BASE}/entries/:id`, async ({ params, request }) => {
    const e = db.entries.find((x) => x.id === Number(params.id));
    if (!e) return err(404, 'not_found', 'Entry not found');
    const b = (await request.json()) as Partial<Entry>;
    if (b.title !== undefined && !b.title.trim()) return err(422, 'invalid_input', 'Title is required');
    Object.assign(e, b, { updated_at: new Date().toISOString() });
    return HttpResponse.json(e);
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
    Object.assign(e, (await request.json()) as object);
    return noContent();
  }),
  http.post(`${BASE}/entries/:id/exclude`, ({ params }) => {
    db.entries = db.entries.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),

  // Today and meeting prep
  http.get(`${BASE}/today`, () => HttpResponse.json(todayPayload())),
  http.get(`${BASE}/meetings/:id/prep`, ({ params }) => HttpResponse.json(db.preps[Number(params.id)] ?? { entry_id: Number(params.id), notes: '', doc_urls: [], meeting_goal: '', priority_id: null })),
  http.put(`${BASE}/meetings/:id/prep`, async ({ params, request }) => {
    const id = Number(params.id);
    db.preps[id] = { entry_id: id, notes: '', doc_urls: [], meeting_goal: '', priority_id: null, ...((await request.json()) as object) };
    return HttpResponse.json(db.preps[id]);
  }),
  http.post(`${BASE}/meetings/:id/prep/draft`, async () => {
    await delay(50);
    return HttpResponse.json({ content: '- Confirm runner capacity ask\n- Share autoscaling results\n- Agree on next steps' });
  }),

  // Priorities and goals
  http.get(`${BASE}/priorities`, ({ request }) => {
    const cycleId = num(new URL(request.url).searchParams.get('cycle_id')) ?? 2;
    const ps = db.priorities.filter((p) => p.cycle_id === cycleId).map(priorityCard);
    const goals = db.goals.filter((g) => g.cycle_id === cycleId).map((g) => ({ goal: g, priorities: ps.filter((p) => p.department_goal_id === g.id) }));
    return HttpResponse.json({
      goals,
      unassigned: ps.filter((p) => p.department_goal_id == null || !db.goals.some((g) => g.id === p.department_goal_id)),
      counts: { active: ps.filter((p) => p.status === 'active').length, done: ps.filter((p) => p.status === 'completed').length, all: ps.length },
    });
  }),
  http.post(`${BASE}/priorities/import`, () => HttpResponse.json({ created: 2, updated: 1, skipped: 1, errors: ['Row 5: missing title'] })),
  http.post(`${BASE}/priorities`, async ({ request }) => {
    const b = (await request.json()) as Record<string, unknown> & { title?: string; cycle_id?: number };
    if (!b.title?.trim()) return err(422, 'invalid_input', 'Title is required');
    const p = {
      id: db.nextId++, cycle_id: b.cycle_id ?? 2, description: null, status: 'active', tracking_status: 'on_track', tier: null, due_date: null, department_goal_id: null, measure_type: null, measure_start: null,
      measure_target: null, measure_current: null, completed_at: null, ...b,
    } as (typeof db.priorities)[number];
    db.priorities.push(p);
    return HttpResponse.json(p, { status: 201 });
  }),
  http.put(`${BASE}/priorities/:id`, async ({ params, request }) => {
    const p = db.priorities.find((x) => x.id === Number(params.id));
    if (!p) return err(404, 'not_found', 'Priority not found');
    Object.assign(p, (await request.json()) as object);
    return HttpResponse.json(p);
  }),
  http.delete(`${BASE}/priorities/:id`, ({ params }) => {
    db.priorities = db.priorities.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),
  http.get(`${BASE}/priorities/:id/updates`, ({ params }) => HttpResponse.json(db.updates.filter((u) => u.priority_id === Number(params.id)).sort((a, b) => (a.created_at < b.created_at ? 1 : -1)))),
  http.post(`${BASE}/priorities/:id/updates`, async ({ params, request }) => {
    const b = (await request.json()) as { tracking_status: never; measure_value: number | null; comment: string; is_blocker: boolean };
    if (!b.comment?.trim()) return err(422, 'invalid_input', 'Say what changed');
    const u = { id: db.nextId++, priority_id: Number(params.id), created_at: new Date().toISOString(), ...b };
    db.updates.push(u);
    const p = db.priorities.find((x) => x.id === Number(params.id));
    if (p) {
      p.tracking_status = b.tracking_status;
      if (b.measure_value != null) p.measure_current = b.measure_value;
    }
    return HttpResponse.json(u, { status: 201 });
  }),
  http.post(`${BASE}/goals`, async ({ request }) => {
    const b = (await request.json()) as { title?: string; cycle_id?: number };
    if (!b.title?.trim()) return err(422, 'invalid_input', 'Title is required');
    const g = { id: db.nextId++, cycle_id: b.cycle_id ?? 2, title: b.title, description: null, status: 'active', completed_at: null };
    db.goals.push(g);
    return HttpResponse.json(g, { status: 201 });
  }),
  http.put(`${BASE}/goals/:id`, async ({ params, request }) => {
    const g = db.goals.find((x) => x.id === Number(params.id));
    if (!g) return err(404, 'not_found', 'Goal not found');
    const b = (await request.json()) as { status?: string };
    Object.assign(g, b);
    if (b.status === 'completed') db.priorities.filter((p) => p.department_goal_id === g.id).forEach((p) => Object.assign(p, { status: 'completed', tracking_status: 'complete' }));
    return HttpResponse.json(g);
  }),
  http.delete(`${BASE}/goals/:id`, ({ params }) => {
    db.goals = db.goals.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),

  // Review
  http.get(`${BASE}/cycles/:id/review`, ({ params }) =>
    HttpResponse.json({ sections: db.sections.map((s) => sectionFor(Number(params.id), s)), entry_count: db.entries.filter((e) => e.cycle_id === Number(params.id)).length }),
  ),
  http.put(`${BASE}/cycles/:id/review/:slug`, async ({ params, request }) => {
    const b = (await request.json()) as { content: string; content_format: 'markdown' | 'blocknote' };
    db.reviews[`${params.id}:${params.slug}`] = { ...b, generated_at: null };
    return HttpResponse.json({ ok: true });
  }),
  http.post(`${BASE}/cycles/:id/review/:slug/draft`, async () => {
    await delay(60);
    return HttpResponse.json({ markdown: '## 1. Drafted example\n\nA first draft from your logbook and priorities.' });
  }),
  http.get(`${BASE}/cycles/:id/archive`, () => HttpResponse.json(db.archive)),

  // Reports
  http.get(`${BASE}/reports`, ({ request }) => {
    const cycleId = num(new URL(request.url).searchParams.get('cycle_id')) ?? 2;
    return HttpResponse.json(
      db.reports.filter((r) => r.cycle_id === cycleId).map(({ id, title, filters, entry_count, generated_at }) => ({ id, title, filters, entry_count, generated_at })).reverse(),
    );
  }),
  http.post(`${BASE}/reports`, async ({ request }) => {
    const b = (await request.json()) as ReportInput;
    const sp = new URLSearchParams(Object.entries({ ...b.filters, cycle_id: b.cycle_id }).filter(([, v]) => v != null && v !== '').map(([k, v]) => [k, String(v)]));
    const list = filterEntries(sp);
    if (list.length === 0) return err(422, 'empty_selection', 'No entries match these filters');
    await delay(30);
    const r = {
      id: db.nextId++, cycle_id: b.cycle_id, title: b.title || 'All projects · report', filters: b.filters, entry_count: list.length, generated_at: new Date().toISOString(), stats: stats(list),
      narrative: { content: b.with_ai ? 'Focused on CI reliability and platform work with several collaborators.' : '', content_format: 'markdown' as const }, key_entries: list.slice(0, 10), ai_error: null,
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
    return HttpResponse.json(trends((sp.get('range') as TrendRange) ?? 'cycle', num(sp.get('cycle_id'))));
  }),

  // Integrations and sync
  http.get(`${BASE}/integrations`, () => HttpResponse.json(db.integrations)),
  http.put(`${BASE}/integrations/:service`, async ({ params, request }) => {
    const i = db.integrations.find((x) => x.service === params.service);
    if (!i) return err(404, 'not_found', 'Unknown service');
    const b = (await request.json()) as { config?: Record<string, string>; token?: string };
    if (b.config) i.config = b.config;
    if (b.token) Object.assign(i, { has_token: true, status: 'connected', last_error: null });
    return noContent();
  }),
  http.post(`${BASE}/integrations/:service/test`, ({ params }) => {
    const i = db.integrations.find((x) => x.service === params.service);
    return HttpResponse.json(i?.status === 'error' ? { ok: false, message: 'Token rejected' } : { ok: true, message: 'Connection works' });
  }),
  http.delete(`${BASE}/integrations/:service/excluded/:id`, ({ params }) => {
    const i = db.integrations.find((x) => x.service === params.service);
    if (i) i.excluded = i.excluded.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),
  http.delete(`${BASE}/integrations/:service`, ({ params }) => {
    const i = db.integrations.find((x) => x.service === params.service);
    if (i) Object.assign(i, { status: 'not_connected', has_token: false, config: {}, last_sync_at: null, last_error: null });
    return noContent();
  }),
  http.post(`${BASE}/sync`, () => {
    db.sync = { running: true, since: Date.now() };
    return new HttpResponse(null, { status: 202 });
  }),
  http.post(`${BASE}/sync/:service`, () => {
    db.sync = { running: true, since: Date.now() };
    return new HttpResponse(null, { status: 202 });
  }),
  http.get(`${BASE}/sync/status`, () => {
    // A mock sync "finishes" after 3 s so the polling behaviour is visible.
    if (db.sync.running && Date.now() - db.sync.since > 3000) db.sync.running = false;
    return HttpResponse.json({
      running: db.sync.running,
      services: Object.fromEntries(db.integrations.map((i) => [i.service, { state: i.status === 'error' ? 'error' : 'idle', message: i.last_error, last_sync_at: i.last_sync_at }])),
      recent_logs: [{ id: 1, service: 'github', status: 'ok', message: '12 new entries', created_at: new Date().toISOString() }],
    });
  }),

  // Settings
  http.get(`${BASE}/settings/profile`, () => HttpResponse.json(db.profile)),
  http.put(`${BASE}/settings/profile`, async ({ request }) => {
    Object.assign(db.profile, await request.json());
    return HttpResponse.json(db.profile);
  }),
  http.get(`${BASE}/settings/people`, () => HttpResponse.json(db.people)),
  http.post(`${BASE}/settings/people`, async ({ request }) => {
    const b = (await request.json()) as { email: string; display_name: string };
    if (!b.email?.includes('@')) return err(422, 'invalid_input', 'Enter a valid email');
    const p = { id: db.nextId++, team: null, relationship_type: null, ...b };
    db.people.push(p);
    return HttpResponse.json(p, { status: 201 });
  }),
  http.delete(`${BASE}/settings/people/:id`, ({ params }) => {
    db.people = db.people.filter((x) => x.id !== Number(params.id));
    return noContent();
  }),
];
