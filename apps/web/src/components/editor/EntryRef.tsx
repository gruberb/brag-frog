import { useOpenEntryRef } from './entry-ref-context';

export function EntryRef({ entryId, label }: { entryId: string; label: string }) {
  const open = useOpenEntryRef();
  return (
    <span
      className="ref-chip"
      role="button"
      tabIndex={0}
      data-entry-ref={entryId}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        open(Number(entryId));
      }}
      onKeyDown={(e) => e.key === 'Enter' && open(Number(entryId))}
    >
      {label || 'Entry'}
    </span>
  );
}
