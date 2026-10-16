import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Select } from '@/components/ui/Select';
import { toast, toastError } from '@/stores/toast';
import type { BulkPatch } from '@/types/api/models';
import { complexityOptions, manualTypeOptions, reachOptions } from '@/utils/entries';
import { useBulkUpdate } from '../api/entries';

type Props = { ids: number[]; priorities: { value: string; label: string }[]; onDone: () => void };

/** Applies one patch to every selected entry. Empty selects mean "leave unchanged". */
export function BulkBar({ ids, priorities, onDone }: Props) {
  const bulk = useBulkUpdate();
  const [priority, setPriority] = useState('');
  const [type, setType] = useState('');
  const [reach, setReach] = useState('');
  const [complexity, setComplexity] = useState('');
  const patch: BulkPatch = {
    ...(priority && { priority_id: priority === 'none' ? null : Number(priority) }),
    ...(type && { entry_type: type }),
    ...(reach && { reach }),
    ...(complexity && { complexity }),
  };
  const empty = Object.keys(patch).length === 0;
  return (
    <div className="card card-b row wrap" style={{ position: 'sticky', bottom: 12, zIndex: 3, boxShadow: 'var(--shadow-md)' }} role="region" aria-label="Bulk edit">
      <b>{ids.length} selected</b>
      <Select aria-label="Set priority" value={priority} onChange={(e) => setPriority(e.target.value)} options={[{ value: 'none', label: 'Remove priority' }, ...priorities]} placeholder="Priority…" />
      <Select aria-label="Set type" value={type} onChange={(e) => setType(e.target.value)} options={manualTypeOptions} placeholder="Type…" />
      <Select aria-label="Set reach" value={reach} onChange={(e) => setReach(e.target.value)} options={reachOptions} placeholder="Reach…" />
      <Select aria-label="Set complexity" value={complexity} onChange={(e) => setComplexity(e.target.value)} options={complexityOptions} placeholder="Complexity…" />
      <span className="grow" />
      <Button onClick={onDone}>Cancel</Button>
      <Button
        variant="primary"
        disabled={empty || ids.length === 0 || bulk.isPending}
        onClick={() =>
          bulk.mutate(
            { ids, patch },
            { onSuccess: () => { toast(`Updated ${ids.length} entries`); onDone(); }, onError: toastError },
          )
        }
      >
        Apply
      </Button>
    </div>
  );
}
