import { sourceMeta } from '@/utils/entries';

export const SourceIcon = ({ source }: { source: string }) => {
  const m = sourceMeta(source);
  return (
    <span className={`src ${m.cls}`} title={m.label} aria-label={m.label}>
      {m.abbr}
    </span>
  );
};
