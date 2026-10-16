import { memo } from 'react';
import type { EntrySummary } from '@/types/api/models';
import { humanize } from '@/utils/entries';
import { SourceIcon } from '@/components/SourceIcon';

const ExternalIcon = () => (
  <svg viewBox="0 0 24 24" aria-hidden>
    <path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" />
  </svg>
);

type Props = {
  entry: EntrySummary;
  onOpen: (id: number) => void;
  selectMode?: boolean;
  selected?: boolean;
  onToggle?: (id: number) => void;
  trailing?: React.ReactNode;
};

/** One quiet line per entry: icon, title, type, date and link. Everything else lives in the drawer. */
export const EntryRow = memo(function EntryRow({ entry, onOpen, selectMode, selected, onToggle, trailing }: Props) {
  const handle = () => (selectMode ? onToggle?.(entry.id) : onOpen(entry.id));
  return (
    <div className={`entry ${selected ? 'sel' : ''}`} role="button" tabIndex={0} onClick={handle} onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), handle())}>
      {selectMode && <input type="checkbox" checked={!!selected} onChange={() => onToggle?.(entry.id)} onClick={(e) => e.stopPropagation()} aria-label={`Select ${entry.title}`} />}
      <SourceIcon source={entry.source} />
      <div className="grow">
        <b className="clamp" style={{ display: 'block' }}>{entry.title}</b>
        <div className="small muted meta">{humanize(entry.entry_type)}</div>
      </div>
      {trailing}
      <span className="date-ext">
        {entry.source_url ? (
          <a href={entry.source_url} target="_blank" rel="noreferrer" onClick={(e) => e.stopPropagation()} aria-label="Open source" style={{ display: 'flex' }}>
            <ExternalIcon />
          </a>
        ) : null}
        {entry.occurred_at}
      </span>
    </div>
  );
});
