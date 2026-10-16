import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { ConfirmAction } from '@/components/ui/ConfirmAction';
import { Select } from '@/components/ui/Select';
import { toast, toastError } from '@/stores/toast';
import type { Entry } from '@/types/api/models';
import { useClassifyEntry, useExcludeEntry } from '../api/entries';

const ROLES = [
  { value: 'manager', label: 'Manager' },
  { value: 'tech_lead', label: 'Tech lead' },
  { value: 'skip_level', label: 'Skip level' },
  { value: 'peer', label: 'Peer' },
  { value: 'stakeholder', label: 'Stakeholder' },
];

/** Meeting classification and "stop syncing" for entries that come from Calendar or Drive. */
export function EntryActions({ entry, onDone }: { entry: Entry; onDone: () => void }) {
  const classify = useClassifyEntry();
  const exclude = useExcludeEntry();
  const [role, setRole] = useState(entry.meeting_role ?? '');
  const [saveRule, setSaveRule] = useState(false);
  const isMeeting = entry.entry_type === 'meeting';
  const excludable = entry.source === 'google_calendar' || entry.source === 'google_drive';
  if (!isMeeting && !excludable) return null;

  return (
    <section className="col" style={{ marginTop: 16, gap: 12 }}>
      {isMeeting && (
        <div className="field">
          <label htmlFor="meeting-role">Meeting role</label>
          <div className="row">
            <Select id="meeting-role" value={role} onChange={(e) => setRole(e.target.value)} options={ROLES} placeholder="Not classified" />
            <Button
              disabled={!role || classify.isPending}
              onClick={() =>
                classify.mutate({ id: entry.id, meeting_role: role, save_rule: saveRule }, { onSuccess: () => toast('Meeting classified'), onError: toastError })
              }
            >
              Save role
            </Button>
          </div>
          {entry.recurring_group && (
            <label className="small row">
              <input type="checkbox" checked={saveRule} onChange={(e) => setSaveRule(e.target.checked)} /> Remember for every meeting in this series
            </label>
          )}
        </div>
      )}
      {excludable && (
        <ConfirmAction
          title={entry.source === 'google_calendar' ? 'Stop syncing this event series' : 'Stop syncing this file'}
          message="Its entries are hidden and it is skipped by future syncs. You can restore it under Settings, Integrations."
          onConfirm={async () => {
            try {
              await exclude.mutateAsync(entry.id);
              onDone();
            } catch (err) {
              toastError(err);
            }
          }}
          trigger={(open) => (
            <Button onClick={open}>{entry.source === 'google_calendar' ? 'Stop syncing this event' : 'Stop syncing this file'}</Button>
          )}
        />
      )}
    </section>
  );
}
