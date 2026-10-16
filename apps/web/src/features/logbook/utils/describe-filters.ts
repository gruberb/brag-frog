import type { EntryFilters } from '@/types/api/models';
import { shortDate } from '@/utils/dates';
import { humanize } from '@/utils/entries';

/** Chips for the active filters, so the report dialog shows exactly what will be reported. */
export function describeFilters(f: EntryFilters, priorityNames: Record<string, string>): string[] {
  const out: string[] = [];
  if (f.from || f.to) out.push(f.from && f.to ? `${shortDate(f.from)} to ${shortDate(f.to)}` : f.from ? `From ${shortDate(f.from)}` : `Until ${shortDate(f.to)}`);
  if (f.priority_id) out.push(priorityNames[String(f.priority_id)] ?? `Priority ${f.priority_id}`);
  if (f.goal_id) out.push(`Goal ${f.goal_id}`);
  if (f.q) out.push(`“${f.q}”`);
  for (const k of ['source', 'category', 'reach', 'complexity', 'role', 'team', 'person'] as const) if (f[k]) out.push(humanize(String(f[k])));
  if (f.no_priority) out.push('No priority');
  if (f.no_team) out.push('No team');
  if (f.no_person) out.push('No person');
  return out;
}

/** Filters the report endpoint understands (`ReportFilters`). Category and the "no X" views are list-only. */
export function toReportFilters(f: EntryFilters) {
  const { from, to, priority_id, goal_id, source, q, team, person, reach, complexity, role } = f;
  return { from, to, priority_id, goal_id, source, q, team, person, reach, complexity, role };
}

export const hasListOnlyFilters = (f: EntryFilters) => !!(f.category || f.no_priority || f.no_team || f.no_person);
