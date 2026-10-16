// Hand-written API types, derived from implementation.md (M1, M2a to M2e).
// They stand in until the ts-rs generated files land in this folder; generated files win on conflict.
// Dates are `YYYY-MM-DD` strings, timestamps are ISO-8601 strings.

export type ApiErrorBody = { error: { code: string; message: string } };

export type PublicConfig = { instance_name: string; allowed_domain_hint: string; public_only: boolean };

export type User = {
  id: number;
  email: string;
  name: string;
  display_name: string | null;
  avatar_url: string | null;
  /** Career level id (matches `clg_levels[].id`); stored as `users.role` in the backend. */
  role: string | null;
};

export type Cycle = {
  id: number;
  name: string;
  start_date: string;
  end_date: string;
  is_active: boolean;
  entry_count: number;
  priority_count: number;
};

export type Me = { user: User; active_cycle: Cycle | null; has_ai: boolean; public_only: boolean };

// ── Entries ──────────────────────────────────────────────────────────────────────────────
export type EntrySource = 'github' | 'phabricator' | 'bugzilla' | 'jira' | 'confluence' | 'google_calendar' | 'google_drive' | 'manual';

export type EntryDecision = { alternatives: string; reasoning: string; outcome: string };

export type Entry = {
  id: number;
  week_id: number;
  source: EntrySource;
  source_id: string | null;
  source_url: string | null;
  title: string;
  description: string;
  entry_type: string;
  status: string | null;
  repository: string | null;
  occurred_at: string;
  teams: string[];
  collaborators: string[];
  outcome_statement: string | null;
  role: string | null;
  reach: string | null;
  complexity: string | null;
  priority_id: number | null;
  meeting_role: string | null;
  recurring_group: string | null;
  start_time: string | null;
  end_time: string | null;
  evidence_urls: string[];
  impact_tags: string[];
  decision: EntryDecision | null;
  created_at: string;
  updated_at: string;
};

/** List projection: no description or decision fields. */
export type EntrySummary = Pick<
  Entry,
  'id' | 'source' | 'source_url' | 'title' | 'entry_type' | 'status' | 'repository' | 'occurred_at' | 'teams' | 'collaborators' | 'priority_id' | 'reach' | 'complexity' | 'role' | 'meeting_role'
>;

export type EntryFilters = {
  cycle_id?: number;
  from?: string;
  to?: string;
  source?: string;
  priority_id?: number;
  goal_id?: number;
  type?: string;
  reach?: string;
  complexity?: string;
  role?: string;
  q?: string;
  team?: string;
  person?: string;
  no_priority?: boolean;
  no_team?: boolean;
  needs_classify?: boolean;
};

export type EntriesPage = { items: EntrySummary[]; next_cursor: string | null; total_matching: number };

export type EntryFacets = {
  teams: string[];
  people: string[];
  sources: string[];
  counts: { total: number; no_priority: number; no_team: number; needs_classify: number };
};

export type EntryInput = Partial<
  Pick<Entry, 'title' | 'description' | 'entry_type' | 'occurred_at' | 'teams' | 'collaborators' | 'priority_id' | 'outcome_statement' | 'role' | 'reach' | 'complexity' | 'evidence_urls' | 'impact_tags' | 'decision'>
> & { cycle_id?: number };

export type BulkPatch = Partial<Pick<Entry, 'priority_id' | 'entry_type' | 'reach' | 'complexity' | 'role'>> & { teams?: string[] };

// ── Today ────────────────────────────────────────────────────────────────────────────────
export type Meeting = {
  entry_id: number;
  title: string;
  start_time: string | null;
  end_time: string | null;
  recurring_group: string | null;
};

export type MeetingDay = { date: string; label: string; is_today: boolean; meetings: Meeting[] };

export type PriorityCard = Priority & { entry_count: number; latest_update: PriorityUpdate | null; blocker_count: number };

export type Today = {
  week: { id: number; week_start: string; week_end: string; iso_week: number; year: number };
  meetings_by_day: MeetingDay[];
  /** entry_id -> prep exists */
  prep_map: Record<string, boolean>;
  active_work: EntrySummary[];
  priorities: PriorityCard[];
  focus_days: { date: string; focus_hours: number; meeting_hours: number }[];
  counts: { meetings: number; focus_hours: number };
};

export type MeetingPrep = {
  entry_id: number;
  notes: string;
  doc_urls: string[];
  meeting_goal: string;
  priority_id: number | null;
};

// ── Priorities ───────────────────────────────────────────────────────────────────────────
export type PriorityStatus = 'not_started' | 'active' | 'on_hold' | 'completed' | 'cancelled';
export type TrackingStatus = 'on_track' | 'progressing' | 'off_track' | 'complete' | 'incomplete' | 'no_update';

export type Priority = {
  id: number;
  title: string;
  description: string | null;
  status: PriorityStatus;
  tracking_status: TrackingStatus | null;
  tier: 'department' | 'team' | 'individual' | null;
  due_date: string | null;
  department_goal_id: number | null;
  measure_type: string | null;
  measure_start: number | null;
  measure_target: number | null;
  measure_current: number | null;
  completed_at: string | null;
};

export type PriorityUpdate = {
  id: number;
  priority_id: number;
  tracking_status: TrackingStatus | null;
  measure_value: number | null;
  comment: string;
  is_blocker: boolean;
  created_at: string;
};

export type DepartmentGoal = { id: number; title: string; description: string | null; status: string; completed_at: string | null };

export type PrioritiesResponse = {
  goals: { goal: DepartmentGoal; priorities: PriorityCard[] }[];
  unassigned: PriorityCard[];
  counts: { active: number; done: number; all: number };
};

export type PriorityInput = Partial<Omit<Priority, 'id' | 'completed_at'>> & { cycle_id?: number };
export type PriorityUpdateInput = { tracking_status: TrackingStatus | null; measure_value: number | null; comment: string; is_blocker: boolean };
export type ImportResult = { created: number; updated: number; skipped: number; errors: string[] };

// ── Review ───────────────────────────────────────────────────────────────────────────────
export type ContentFormat = 'markdown' | 'blocknote';

export type ReviewSection = {
  slug: string;
  title: string;
  question: string;
  form_question: string;
  form_guidance: string;
  form_bullets: string[];
  form_tip: string;
  form_placeholder: string;
  focus_priorities: boolean;
  content: string;
  content_format: ContentFormat;
  generated_at: string | null;
};

export type ReviewResponse = { sections: ReviewSection[]; entry_count: number };

export type Archive = {
  checkins: { id: number; title: string; period: string; content: string }[];
  contribution_examples: { id: number; title: string; content: string; entry_titles: string[] }[];
  status_updates: { id: number; title: string; content: string; created_at: string }[];
  last_week_reports: { id: number; title: string; content: string; created_at: string }[];
};

// ── Reports ──────────────────────────────────────────────────────────────────────────────
export type ReportFilters = Omit<EntryFilters, 'cycle_id' | 'needs_classify'>;

export type ReportStats = {
  entries: number;
  people: number;
  repos: number;
  teams: number;
  per_week: { week_start: string; count: number }[];
  per_project: { priority_id: number | null; name: string; count: number }[];
  people_top: { name: string; count: number }[];
  repos_top: { name: string; count: number }[];
  teams_top: { name: string; count: number }[];
};

export type ReportMeta = { id: number; title: string; filters: ReportFilters; entry_count: number; generated_at: string };

export type Report = ReportMeta & {
  stats: ReportStats;
  narrative: { content: string; content_format: ContentFormat };
  key_entries: Entry[];
  ai_error?: string | null;
};

export type ReportInput = { cycle_id: number; title?: string; filters: ReportFilters; with_ai: boolean };

// ── Trends ───────────────────────────────────────────────────────────────────────────────
export type TrendRange = '4w' | 'quarter' | 'cycle';

export type Trends = {
  kpis: { entries: number; entries_delta_pct: number | null; people: number; repos: number; priority_linked_pct: number };
  weekly: { week_start: string; by_type: { code: number; review: number; meeting: number; other: number } }[];
  priority_share: { priority_id: number | null; name: string; count: number }[];
  people: { name: string; count: number }[];
  repos: { name: string; count: number }[];
  teams: { name: string; count: number }[];
  weekday_hour: { weekday: number; hour: number | null; count: number }[] | null;
};

// ── Settings, config, integrations ───────────────────────────────────────────────────────
export type Profile = {
  display_name: string | null;
  role: string | null;
  team: string | null;
  manager_name: string | null;
  skip_level_name: string | null;
  direct_reports: string | null;
  timezone: string | null;
  week_start: string | null;
  work_start_time: string | null;
  work_end_time: string | null;
  wants_promotion: boolean;
};

export type PeopleAlias = { id: number; email: string; display_name: string; team: string | null; relationship_type: string | null };
export type PeopleAliasInput = { email: string; display_name: string; team?: string | null };

export type ClgLevel = {
  id: string;
  title: string;
  one_liner: string;
  problems: Record<string, string>;
  people: Record<string, string>;
};

export type ServiceMeta = {
  service: string;
  label: string;
  token_url: string | null;
  note: string | null;
  base_url_placeholder: string | null;
  email_placeholder: string | null;
  org_placeholder: string | null;
  default_orgs: string | null;
  default_base_url: string | null;
};

export type ReviewSectionConfig = Omit<ReviewSection, 'content' | 'content_format' | 'generated_at'>;

export type AppConfig = {
  instance_name: string;
  review_sections: ReviewSectionConfig[];
  services: ServiceMeta[];
  clg_levels: ClgLevel[];
  atlassian_base_url: string | null;
};

export type IntegrationStatus = 'connected' | 'not_connected' | 'error';

export type Integration = {
  service: string;
  status: IntegrationStatus;
  last_sync_at: string | null;
  last_error: string | null;
  entry_count: number;
  config: Record<string, string>;
  has_token: boolean;
  excluded: { id: number; label: string }[];
};

export type IntegrationInput = { config?: Record<string, string>; token?: string };

export type SyncStatus = {
  running: boolean;
  services: Record<string, { state: 'idle' | 'running' | 'error'; message: string | null; last_sync_at: string | null }>;
  recent_logs: { id: number; service: string; status: string; message: string | null; created_at: string }[];
};

export type CycleBundle = {
  cycle: Cycle;
  review: { slug: string; title: string; content: string; content_format: ContentFormat }[];
  goals: DepartmentGoal[];
  priorities: (Priority & { updates: PriorityUpdate[] })[];
  weeks: { week_start: string; iso_week: number; entries: Entry[] }[];
};

export type CycleInput = { name: string; start_date: string; end_date: string; activate: boolean };
