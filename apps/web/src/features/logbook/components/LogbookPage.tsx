import { useState, type ReactNode } from 'react';
import { Button } from '@/components/ui/Button';
import type { Option } from '@/components/ui/Select';
import type { EntryFacets } from '@/types/api/models';
import { FilterBar } from './FilterBar';
import { ReportDialog } from './ReportDialog';

type Props = {
  cycleName: string;
  cycleEntryCount: number | undefined;
  matching: number;
  facets?: EntryFacets;
  goals: Option[];
  priorities: Option[];
  readOnly: boolean;
  selectMode: boolean;
  onToggleSelect: () => void;
  onLogEntry: () => void;
  /** The entry list and bulk bar come from features/entries, composed by the route. */
  children: ReactNode;
};

export function LogbookPage({ cycleName, cycleEntryCount, matching, facets, goals, priorities, readOnly, selectMode, onToggleSelect, onLogEntry, children }: Props) {
  const [report, setReport] = useState(false);
  const priorityNames = Object.fromEntries(priorities.map((p) => [p.value, p.label]));
  return (
    <>
      <div className="page-head">
        <div>
          <h1>Logbook</h1>
          <p>
            {cycleName} · {cycleEntryCount == null ? `${matching}` : `${matching} of ${cycleEntryCount}`} entries
          </p>
        </div>
        <div className="head-actions">
          <div className="row">
            {!readOnly && (
              <Button variant="toggle" onClick={onToggleSelect} aria-pressed={selectMode}>
                {selectMode ? 'Done selecting' : 'Bulk edit'}
              </Button>
            )}
            <Button variant="transfer" icon="⬇" onClick={() => setReport(true)}>Generate report</Button>
          </div>
          {!readOnly && <Button variant="primary" onClick={onLogEntry}>+ Log entry</Button>}
        </div>
      </div>
      <FilterBar facets={facets} goals={goals} priorities={priorities} />
      <div style={{ marginTop: 4 }}>{children}</div>
      <ReportDialog open={report} onOpenChange={setReport} matching={matching} priorityNames={priorityNames} />
    </>
  );
}
