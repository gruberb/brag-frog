import type { Trends } from '@/types/api/models';

type Totals = Trends['weekly'][number]['by_category'];

/** One series per category, in the API's order (`worklog::model::CATEGORIES`). Colours come from the design tokens only. */
export const CATEGORY_SERIES: readonly { key: keyof Totals; label: string; color: string }[] = [
  { key: 'code', label: 'Code', color: 'var(--color-primary)' },
  { key: 'reviews', label: 'Reviews', color: 'var(--color-blue-30)' },
  { key: 'tickets', label: 'Tickets', color: 'var(--color-yellow-30)' },
  { key: 'docs', label: 'Docs', color: 'var(--color-green-30)' },
  { key: 'meetings', label: 'Meetings', color: 'var(--color-purple-30)' },
  { key: 'collaboration', label: 'Collaboration', color: 'var(--color-blue-60)' },
  { key: 'learning', label: 'Learning', color: 'var(--color-orange-40)' },
  { key: 'other', label: 'Other', color: 'var(--color-gray-40)' },
];
