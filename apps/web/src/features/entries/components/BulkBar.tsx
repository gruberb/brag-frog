import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Select } from '@/components/ui/Select';
import { toast, toastError } from '@/stores/toast';
import type { BulkRequest } from '@/types/api/models';
import { complexityOptions, reachOptions, roleOptions } from '@/utils/entries';
import { useBulkUpdate } from '../api/entries';

type Props = { ids: number[]; priorities: { value: string; label: string }[]; onDone: () => void };

/** Applies one patch to every selected entry. Empty fields mean "leave unchanged"; teams are appended, not replaced. */
export function BulkBar({ ids, priorities, onDone }: Props) {
  const bulk = useBulkUpdate();
  const [priority, setPriority] = useState('');
  const [reach, setReach] = useState('');
  const [complexity, setComplexity] = useState('');
  const [role, setRole] = useState('');
  const [teams, setTeams] = useState('');
  const teamList = teams.split(',').map((t) => t.trim()).filter(Boolean);
  const patch: Omit<BulkRequest, 'ids'> = {
    ...(priority && { priority_id: Number(priority) }),
    ...(reach && { reach }),
    ...(complexity && { complexity }),
    ...(role && { role }),
    ...(teamList.length > 0 && { teams: teamList, merge_mode: 'append' }),
  };
  const empty = Object.keys(patch).length === 0;
  return (
    <div className="card card-b row wrap" style={{ position: 'sticky', bottom: 12, zIndex: 3, boxShadow: 'var(--shadow-md)' }} role="region" aria-label="Bulk edit">
      <b>{ids.length === 0 ? 'Click entries to select them' : `${ids.length} selected`}</b>
      <Select aria-label="Set priority" value={priority} onChange={(e) => setPriority(e.target.value)} options={priorities} placeholder="Priority…" />
      <Select aria-label="Set reach" value={reach} onChange={(e) => setReach(e.target.value)} options={reachOptions} placeholder="Reach…" />
      <Select aria-label="Set complexity" value={complexity} onChange={(e) => setComplexity(e.target.value)} options={complexityOptions} placeholder="Complexity…" />
      <Select aria-label="Set role" value={role} onChange={(e) => setRole(e.target.value)} options={roleOptions} placeholder="Role…" />
      <input aria-label="Add teams" placeholder="Add teams…" value={teams} onChange={(e) => setTeams(e.target.value)} style={{ width: 140 }} />
      <span className="grow" />
      <Button onClick={onDone}>Cancel</Button>
      <Button
        variant="primary"
        disabled={empty || ids.length === 0 || bulk.isPending}
        onClick={() =>
          bulk.mutate(
            { ids, ...patch },
            { onSuccess: () => { toast(`Updated ${ids.length} entries`); onDone(); }, onError: toastError },
          )
        }
      >
        Apply
      </Button>
    </div>
  );
}
