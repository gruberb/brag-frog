import { useEffect } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router';
import { EntryDrawer } from '@/features/entries/components/EntryDrawer';
import { EvidencePicker } from '@/features/entries/components/EvidencePicker';
import { searchEntries } from '@/features/entries/api/search';
import { ArchivePage } from '@/features/review/components/ArchivePage';
import { ReviewPage, type EntryDrawerSlot, type EvidenceSlot } from '@/features/review/components/ReviewPage';
import { paths } from '@/config/paths';
import { EmptyState } from '@/components/ui/EmptyState';
import { useCycleContext } from '@/stores/session';

const Evidence: EvidenceSlot = ({ open, onOpenChange, onInsert }) => (
  <EvidencePicker open={open} onOpenChange={onOpenChange} onInsert={(e) => onInsert({ id: e.id, title: e.title })} />
);
const EntryDrawerView: EntryDrawerSlot = ({ entryId, onClose }) => <EntryDrawer entryId={entryId} onClose={onClose} />;

/** Composes the review (features/review) with evidence picking and the entry drawer (features/entries). */
export function Component() {
  const { cycle, activeCycle, selectCycle } = useCycleContext();
  const { cycleId: cycleParam, section } = useParams();
  const nav = useNavigate();
  const [sp] = useSearchParams();
  const cycleId = Number(cycleParam) || cycle?.id || null;

  // A deep link to a past cycle's review switches the app-wide cycle so the banner and pickers agree.
  useEffect(() => {
    if (cycleId && cycle && cycleId !== cycle.id) selectCycle(cycleId === activeCycle?.id ? null : cycleId);
  }, [cycleId, cycle, activeCycle, selectCycle]);

  if (!cycleId) return <EmptyState title="No cycle yet">Create one in Settings to start your review.</EmptyState>;
  if (section === 'archive') return <ArchivePage cycleId={cycleId} />;

  return (
    <ReviewPage
      cycleId={cycleId}
      section={section}
      readOnly={cycleId !== activeCycle?.id}
      onSectionChange={(slug) => void nav({ pathname: paths.review(cycleId, slug), search: sp.toString() ? `?${sp}` : '' })}
      searchEntries={(q) => searchEntries(cycleId, q)}
      Evidence={Evidence}
      EntryDrawerView={EntryDrawerView}
    />
  );
}
