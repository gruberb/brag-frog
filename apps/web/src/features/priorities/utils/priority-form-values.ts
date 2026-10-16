import { z } from 'zod';
import type { Priority, PriorityInput } from '@/types/api/models';

const optionalNumber = z.string().refine((v) => v === '' || !Number.isNaN(Number(v)), 'Enter a number');

export const priorityFormSchema = z.object({
  title: z.string().trim().min(1, 'Title is required').max(200),
  description: z.string(),
  department_goal_id: z.string(),
  tier: z.string(),
  due_date: z.string().refine((v) => v === '' || /^\d{4}-\d{2}-\d{2}$/.test(v), 'Use a valid date'),
  status: z.string(),
  measure_type: z.string(),
  measure_start: optionalNumber,
  measure_target: optionalNumber,
  measure_current: optionalNumber,
});
export type PriorityFormValues = z.infer<typeof priorityFormSchema>;

const n = (v: number | null | undefined) => (v == null ? '' : String(v));

export const toPriorityFormValues = (p?: Partial<Priority>): PriorityFormValues => ({
  title: p?.title ?? '',
  description: p?.description ?? '',
  department_goal_id: p?.department_goal_id ? String(p.department_goal_id) : '',
  tier: p?.tier ?? '',
  due_date: p?.due_date ?? '',
  status: p?.status ?? 'active',
  measure_type: p?.measure_type ?? '',
  measure_start: n(p?.measure_start),
  measure_target: n(p?.measure_target),
  measure_current: n(p?.measure_current),
});

const num = (v: string) => (v === '' ? null : Number(v));

export const toPriorityInput = (v: PriorityFormValues): PriorityInput => ({
  title: v.title.trim(),
  description: v.description || null,
  department_goal_id: v.department_goal_id ? Number(v.department_goal_id) : null,
  tier: (v.tier || null) as PriorityInput['tier'],
  due_date: v.due_date || null,
  status: v.status as PriorityInput['status'],
  measure_type: v.measure_type || null,
  measure_start: num(v.measure_start),
  measure_target: num(v.measure_target),
  measure_current: num(v.measure_current),
});
