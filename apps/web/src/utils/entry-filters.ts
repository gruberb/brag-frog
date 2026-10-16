import type { EntryFilters } from '@/types/api/models';

const STRING_KEYS = ['from', 'to', 'source', 'category', 'reach', 'complexity', 'role', 'q', 'team', 'person'] as const;
const NUM_KEYS = ['priority_id', 'goal_id'] as const;
const BOOL_KEYS = ['no_priority', 'no_team', 'no_person'] as const;

/** URL search params are the single source of truth for Logbook filters, so views are linkable. */
export function parseFilters(sp: URLSearchParams): EntryFilters {
  const f: EntryFilters = {};
  for (const k of STRING_KEYS) {
    const v = sp.get(k);
    if (v) f[k] = v;
  }
  for (const k of NUM_KEYS) {
    const v = Number(sp.get(k));
    if (v) f[k] = v;
  }
  for (const k of BOOL_KEYS) if (sp.get(k) === '1' || sp.get(k) === 'true') f[k] = true;
  return f;
}

export function writeFilters(sp: URLSearchParams, patch: Partial<EntryFilters>): URLSearchParams {
  const next = new URLSearchParams(sp);
  for (const [k, v] of Object.entries(patch)) {
    if (v === undefined || v === null || v === '' || v === false) next.delete(k);
    else next.set(k, v === true ? '1' : String(v));
  }
  return next;
}

export const FILTER_KEYS: string[] = [...STRING_KEYS, ...NUM_KEYS, ...BOOL_KEYS];
export const activeFilterCount = (f: EntryFilters) => Object.keys(f).length;
