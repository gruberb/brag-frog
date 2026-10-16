import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api-client';
import type { EntryTypeDto } from '@/types/api/models';

/** Entry type metadata from `GET /meta/entry-types`: manual types feed the entry form, all types label synced rows. */
export function useEntryTypes() {
  const q = useQuery({ queryKey: ['entry-types'], queryFn: ({ signal }) => api<EntryTypeDto[]>('/meta/entry-types', { signal }), staleTime: Infinity });
  const opt = (t: EntryTypeDto) => ({ value: t.value, label: t.label });
  return { all: q.data?.map(opt) ?? [], manual: q.data?.filter((t) => t.manual).map(opt) ?? [], isLoading: q.isLoading };
}
