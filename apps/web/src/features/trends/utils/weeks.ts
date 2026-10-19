import { addDays, format, parseISO } from 'date-fns';
import type { Trends } from '@/types/api/models';

export const weekTotal = (w: Trends['weekly'][number]) => Object.values(w.by_category).reduce((n, c) => n + c, 0);
export const weekRange = (start: string) => ({ from: start, to: format(addDays(parseISO(start), 6), 'yyyy-MM-dd') });
