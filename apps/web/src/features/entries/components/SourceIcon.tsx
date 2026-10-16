import type { EntrySource } from '@/types/api/models';
import { sourceMeta } from '@/utils/entries';

export const SourceIcon = ({ source }: { source: EntrySource }) => {
  const m = sourceMeta[source] ?? sourceMeta.manual;
  return (
    <span className={`src ${m.cls}`} title={m.label} aria-label={m.label}>
      {m.abbr}
    </span>
  );
};
