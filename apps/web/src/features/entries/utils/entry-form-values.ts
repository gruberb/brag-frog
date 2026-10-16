import { z } from 'zod';
import type { Entry, EntryInput } from '@/types/api/models';

export const entryFormSchema = z.object({
  title: z.string().trim().min(1, 'Title is required').max(300, 'Keep the title under 300 characters'),
  entry_type: z.string(),
  occurred_at: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, 'Use a valid date'),
  priority_id: z.string(),
  teams: z.string(),
  collaborators: z.string(),
  reach: z.string(),
  complexity: z.string(),
  role: z.string(),
  description: z.string(),
  outcome_statement: z.string(),
});
export type EntryFormValues = z.infer<typeof entryFormSchema>;

const list = (s: string) => s.split(',').map((x) => x.trim()).filter(Boolean);

export function toFormValues(e: Partial<Entry> & { title?: string }, today: string): EntryFormValues {
  return {
    title: e.title ?? '',
    entry_type: e.entry_type ?? 'development',
    occurred_at: e.occurred_at ?? today,
    priority_id: e.priority_id ? String(e.priority_id) : '',
    teams: (e.teams ?? []).join(', '),
    collaborators: (e.collaborators ?? []).join(', '),
    reach: e.reach ?? '',
    complexity: e.complexity ?? '',
    role: e.role ?? '',
    description: e.description ?? '',
    outcome_statement: e.outcome_statement ?? '',
  };
}

export function toEntryInput(v: EntryFormValues): EntryInput {
  return {
    title: v.title.trim(),
    entry_type: v.entry_type,
    occurred_at: v.occurred_at,
    priority_id: v.priority_id ? Number(v.priority_id) : null,
    teams: list(v.teams),
    collaborators: list(v.collaborators),
    reach: v.reach || null,
    complexity: v.complexity || null,
    role: v.role || null,
    description: v.description,
    outcome_statement: v.outcome_statement || null,
  };
}

