import { useEffect, useRef, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { Select } from '@/components/ui/Select';
import { usePriorityOptions } from '@/hooks/use-priority-options';
import { useCycleContext } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import { useEntryTypes } from '@/hooks/use-entry-types';
import { isoDay } from '@/utils/dates';
import { useCreateEntry, useEntryFacets } from '../api/entries';
import { EntryForm } from './EntryForm';
import { toFormValues } from '../utils/entry-form-values';

export const FOCUS_COMPOSER_EVENT = 'bf:focus-composer';

/** Full composer in a dialog: "Log entry" in the Logbook and Shift+Enter from the quick composer. */
export function ComposerDialog({ open, onOpenChange, initialTitle = '' }: { open: boolean; onOpenChange: (o: boolean) => void; initialTitle?: string }) {
  const { cycle } = useCycleContext();
  const { priorities } = usePriorityOptions(cycle?.id);
  const facets = useEntryFacets(cycle?.id ?? null);
  const create = useCreateEntry();
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="Log entry">
      <div className="card-b">
        <EntryForm
          defaults={toFormValues({ title: initialTitle }, isoDay(new Date()))}
          priorities={priorities}
          teamSuggestions={facets.data?.teams}
          peopleSuggestions={facets.data?.people}
          submitLabel="Log entry"
          onCancel={() => onOpenChange(false)}
          onSubmit={async (input) => {
            try {
              await create.mutateAsync(input);
              toast('Entry logged');
              onOpenChange(false);
            } catch (err) {
              toastError(err);
            }
          }}
        />
      </div>
    </Dialog>
  );
}

/**
 * One-line composer (Today). Enter logs, Shift+Enter opens the details dialog.
 * Type, priority, team and people are optional and live behind compact selects to keep the line quiet.
 */
export function QuickComposer() {
  const { cycle, readOnly } = useCycleContext();
  const { priorities } = usePriorityOptions(cycle?.id);
  const create = useCreateEntry();
  const types = useEntryTypes();
  const [title, setTitle] = useState('');
  const [type, setType] = useState('development');
  const [priority, setPriority] = useState('');
  const [details, setDetails] = useState(false);
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const focus = () => ref.current?.focus();
    window.addEventListener(FOCUS_COMPOSER_EVENT, focus);
    return () => window.removeEventListener(FOCUS_COMPOSER_EVENT, focus);
  }, []);

  if (readOnly) return null;

  const submit = async () => {
    const t = title.trim();
    if (!t) return;
    try {
      await create.mutateAsync({ title: t, entry_type: type, priority_id: priority ? Number(priority) : undefined, occurred_at: isoDay(new Date()) });
      setTitle('');
    } catch (err) {
      toastError(err);
    }
  };

  return (
    <>
      <div className="col" style={{ gap: 8 }}>
        <div className="row">
          <input
            ref={ref}
            className="grow"
            style={{ fontSize: 15, padding: '10px 12px' }}
            placeholder="Log something you did…  (⌘J)"
            aria-label="Log something you did"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== 'Enter') return;
              e.preventDefault();
              if (e.shiftKey) setDetails(true);
              else void submit();
            }}
          />
          <Button variant="primary" onClick={() => void submit()} disabled={!title.trim() || create.isPending}>
            Log
          </Button>
        </div>
        <div className="row wrap">
          <Select aria-label="Type" value={type} onChange={(e) => setType(e.target.value)} options={types.manual} style={{ padding: '3px 8px', fontSize: 12 }} />
          <Select aria-label="Priority" value={priority} onChange={(e) => setPriority(e.target.value)} options={priorities} placeholder="No priority" style={{ padding: '3px 8px', fontSize: 12 }} />
          <span className="small muted">Enter logs · Shift+Enter adds details</span>
        </div>
      </div>
      <ComposerDialog open={details} onOpenChange={(o) => { setDetails(o); if (!o) setTitle(''); }} initialTitle={title} />
    </>
  );
}
