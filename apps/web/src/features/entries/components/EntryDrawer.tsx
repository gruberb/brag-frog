import { useMemo } from 'react';
import { Drawer } from '@/components/ui/Drawer';
import { SkeletonRows } from '@/components/ui/Spinner';
import { ErrorState } from '@/components/ui/EmptyState';
import { ConfirmAction } from '@/components/ui/ConfirmAction';
import { Button } from '@/components/ui/Button';
import { usePriorityOptions } from '@/hooks/use-priority-options';
import { useCycleContext } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import { useDeleteEntry, useEntry, useEntryFacets, useUpdateEntry } from '../api/entries';
import { EntryForm } from './EntryForm';
import { toFormValues } from '../utils/entry-form-values';

type Props = { entryId: number | null; onClose: () => void };

/** Edit form for one entry. Replaces the inline forms of the old logbook. */
export function EntryDrawer({ entryId, onClose }: Props) {
  const { cycle, readOnly } = useCycleContext();
  const entry = useEntry(entryId);
  const { priorities } = usePriorityOptions(cycle?.id);
  const facets = useEntryFacets(cycle?.id ?? null);
  const update = useUpdateEntry();
  const del = useDeleteEntry();
  const defaults = useMemo(() => (entry.data ? toFormValues(entry.data, '') : null), [entry.data]);
  const e = entry.data;

  return (
    <Drawer open={entryId != null} onOpenChange={(o) => !o && onClose()} title={e?.title ?? 'Entry'} subtitle={e ? `${e.source.replace('_', ' ')} · ${e.occurred_at}` : undefined}>
      {entry.isPending && <SkeletonRows rows={4} h={40} />}
      {entry.error && <ErrorState error={entry.error} onRetry={() => entry.refetch()} />}
      {e && defaults && (
        <EntryForm
          defaults={defaults}
          priorities={priorities}
          teamSuggestions={facets.data?.teams}
          peopleSuggestions={facets.data?.people}
          synced={e.source !== 'manual'}
          readOnly={readOnly}
          onCancel={onClose}
          onSubmit={async (input) => {
            try {
              await update.mutateAsync({ id: e.id, input });
              toast('Entry saved');
              onClose();
            } catch (err) {
              toastError(err);
            }
          }}
          footerExtra={
            !readOnly && (
              <ConfirmAction
                title="Delete entry"
                message={e.source === 'manual' ? 'This entry will be deleted permanently.' : 'This synced entry will be hidden and will not return on the next sync.'}
                onConfirm={async () => {
                  try {
                    await del.mutateAsync(e.id);
                    onClose();
                  } catch (err) {
                    toastError(err);
                  }
                }}
                trigger={(open) => (
                  <Button variant="danger" onClick={open}>
                    Delete
                  </Button>
                )}
              />
            )
          }
        />
      )}
      {e?.source_url && (
        <a className="link small" href={e.source_url} target="_blank" rel="noreferrer">
          Open source ↗
        </a>
      )}
    </Drawer>
  );
}
