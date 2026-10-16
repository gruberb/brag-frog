import { useState } from 'react';
import { useSearchParams } from 'react-router';
import { useEntries, useEntryFacets } from '@/features/entries/api/entries';
import { BulkBar } from '@/features/entries/components/BulkBar';
import { ComposerDialog } from '@/features/entries/components/Composer';
import { EntryDrawer } from '@/features/entries/components/EntryDrawer';
import { EntryStream } from '@/features/entries/components/EntryStream';
import { LogbookPage } from '@/features/logbook/components/LogbookPage';
import { usePriorityOptions } from '@/hooks/use-priority-options';
import { useCycleContext } from '@/stores/session';
import { parseFilters } from '@/utils/entry-filters';

/** Composes the logbook chrome (features/logbook) with the entry list, drawer and composer (features/entries). */
export function Component() {
  const { cycle, readOnly } = useCycleContext();
  const [sp, setSp] = useSearchParams();
  const filters = parseFilters(sp);
  const query = useEntries(cycle?.id ?? null, filters);
  const facets = useEntryFacets(cycle?.id ?? null);
  const { goals, priorities } = usePriorityOptions(cycle?.id);
  const [selectMode, setSelectMode] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [composer, setComposer] = useState(false);
  const openId = Number(sp.get('entry')) || null;

  const setOpen = (id: number | null) =>
    setSp((prev) => {
      const next = new URLSearchParams(prev);
      if (id) next.set('entry', String(id));
      else next.delete('entry');
      return next;
    });
  const toggle = (id: number) => setSelected((s) => { const n = new Set(s); if (n.has(id)) n.delete(id); else n.add(id); return n; });
  const exitSelect = () => { setSelectMode(false); setSelected(new Set()); };

  return (
    <>
      <LogbookPage
        cycleName={cycle?.name ?? 'No cycle'}
        cycleEntryCount={cycle?.entry_count ?? 0}
        matching={query.data?.pages[0]?.total_matching ?? 0}
        facets={facets.data}
        goals={goals}
        priorities={priorities}
        readOnly={readOnly}
        selectMode={selectMode}
        onToggleSelect={() => (selectMode ? exitSelect() : setSelectMode(true))}
        onLogEntry={() => setComposer(true)}
      >
        <EntryStream query={query} onOpen={setOpen} selectMode={selectMode} selected={selected} onToggle={toggle} />
        {selectMode && <BulkBar ids={[...selected]} priorities={priorities} onDone={exitSelect} />}
      </LogbookPage>
      <EntryDrawer entryId={openId} onClose={() => setOpen(null)} />
      <ComposerDialog open={composer} onOpenChange={setComposer} />
    </>
  );
}
