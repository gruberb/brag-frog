import type { EntryFilters, ReportFilters } from '@/types/api/models';
import { shortDate } from '@/utils/dates';
import { humanize } from '@/utils/entries';

/** Chips for the active filters, so the dialog shows exactly what will be reported. */
export function describeFilters(f: EntryFilters | ReportFilters, priorityNames: Record<string, string>): string[] {
  const out: string[] = [];
  if (f.from || f.to) out.push(f.from && f.to ? `${shortDate(f.from)} to ${shortDate(f.to)}` : f.from ? `From ${shortDate(f.from)}` : `Until ${shortDate(f.to)}`);
  if (f.priority_id) out.push(priorityNames[String(f.priority_id)] ?? `Priority ${f.priority_id}`);
  if (f.goal_id) out.push(`Goal ${f.goal_id}`);
  if (f.q) out.push(`“${f.q}”`);
  for (const k of ['source', 'type', 'reach', 'complexity', 'role', 'team', 'person'] as const) if (f[k]) out.push(humanize(String(f[k])));
  if (f.no_priority) out.push('No priority');
  if (f.no_team) out.push('No team');
  return out;
}

