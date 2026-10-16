import { z } from 'zod';
import type { CreatePriorityRequest, Priority, UpdatePriorityRequest } from '@/types/api/models';

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

const text = (v: string) => (v === '' ? undefined : v);
const numOrUndef = (v: string) => (v === '' ? undefined : Number(v));

/** Create: empty fields are omitted (the API has no nulls here). */
export const toCreateRequest = (v: PriorityFormValues): CreatePriorityRequest => ({
  title: v.title.trim(),
  description: text(v.description),
  department_goal_id: numOrUndef(v.department_goal_id),
  tier: text(v.tier),
  due_date: text(v.due_date),
  status: v.status,
  measure_type: text(v.measure_type),
  measure_start: numOrUndef(v.measure_start),
  measure_target: numOrUndef(v.measure_target),
});

/** Update: empty fields are sent as null so they can be cleared. */
export const toUpdateRequest = (v: PriorityFormValues): UpdatePriorityRequest => ({
  title: v.title.trim(),
  description: v.description || null,
  department_goal_id: v.department_goal_id ? Number(v.department_goal_id) : null,
  tier: v.tier || null,
  due_date: v.due_date || null,
  status: v.status,
  measure_type: v.measure_type || null,
  measure_start: num(v.measure_start),
  measure_target: num(v.measure_target),
  measure_current: num(v.measure_current),
});
