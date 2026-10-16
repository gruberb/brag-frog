import { format, parseISO, isToday, isYesterday, differenceInCalendarDays, startOfISOWeek } from 'date-fns';

/** `YYYY-MM-DD` to a local Date without timezone drift. */
export const parseDay = (d: string) => parseISO(d.length === 10 ? `${d}T00:00:00` : d);
export const isoDay = (d: Date) => format(d, 'yyyy-MM-dd');

export function dayHeading(d: string, now = new Date()): string {
  const date = parseDay(d);
  if (isToday(date)) return 'Today';
  if (isYesterday(date)) return 'Yesterday';
  if (differenceInCalendarDays(now, date) < 7 && differenceInCalendarDays(now, date) > 0) return format(date, 'EEEE, MMMM d');
  return format(date, date.getFullYear() === now.getFullYear() ? 'EEEE, MMMM d' : 'EEEE, MMMM d, yyyy');
}

export const weekKey = (d: string) => isoDay(startOfISOWeek(parseDay(d)));
export const shortDate = (d: string | null | undefined) => (d ? format(parseDay(d), 'MMM d') : '');

export function relativeTime(iso: string | null | undefined, now = new Date()): string {
  if (!iso) return 'never';
  const mins = Math.round((now.getTime() - new Date(iso).getTime()) / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins} min ago`;
  const h = Math.round(mins / 60);
  if (h < 24) return `${h} h ago`;
  return `${Math.round(h / 24)} d ago`;
}

export function daysLeft(end: string, now = new Date()): number {
  return differenceInCalendarDays(parseDay(end), now);
}
