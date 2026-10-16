import type { Priority, TodayPriority } from '@/types/api/models';
import type { Tone } from '@/components/ui/Badge';

type Meta = { label: string; tone: Tone; bar: string };

export const TRACKING: Record<string, Meta> = {
  on_track: { label: 'On track', tone: 'green', bar: 'var(--color-success)' },
  progressing: { label: 'Progressing', tone: 'blue', bar: 'var(--color-blue-30)' },
  off_track: { label: 'At risk', tone: 'yellow', bar: 'var(--color-warning)' },
  complete: { label: 'Completed', tone: 'gray', bar: 'var(--color-success)' },
  incomplete: { label: 'Incomplete', tone: 'gray', bar: 'var(--color-gray-40)' },
  no_update: { label: 'No update', tone: 'gray', bar: 'var(--color-gray-40)' },
};

/** Unknown tracking values fall back to "No update" so a new backend value cannot break the row. */
export const trackingMeta = (s: string | null | undefined): Meta => TRACKING[s ?? 'no_update'] ?? TRACKING.no_update;

export const trackingOptions = Object.entries(TRACKING).map(([value, m]) => ({ value, label: m.label }));

export const isDonePriority = (p: Pick<Priority, 'status'>) => p.status === 'completed' || p.status === 'cancelled';

type Measured = Pick<Priority, 'measure_start' | 'measure_target' | 'measure_current' | 'status'> | Pick<TodayPriority, 'measure_start' | 'measure_target' | 'measure_current' | 'status'>;

/** Progress from the measure fields, 0 to 100; null when the priority has no measure. */
export function progressPct(p: Measured): number | null {
  if (p.status === 'completed') return 100;
  const { measure_start: s, measure_target: t, measure_current: c } = p;
  if (s == null || t == null || c == null || s === t) return null;
  return Math.max(0, Math.min(100, Math.round(((c - s) / (t - s)) * 100)));
}
