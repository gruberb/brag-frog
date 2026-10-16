import { addDays, format, parseISO } from 'date-fns';
import type { Trends } from '@/types/api/models';

export const weekTotal = (w: Trends['weekly'][number]) => w.by_type.code + w.by_type.review + w.by_type.meeting + w.by_type.other;
export const weekRange = (start: string) => ({ from: start, to: format(addDays(parseISO(start), 6), 'yyyy-MM-dd') });
