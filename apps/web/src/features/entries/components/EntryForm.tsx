import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Button } from '@/components/ui/Button';
import { Select } from '@/components/ui/Select';
import { allTypeOptions, complexityOptions, manualTypeOptions, reachOptions, roleOptions } from '@/utils/entries';
import type { EntryInput } from '@/types/api/models';
import { entryFormSchema, toEntryInput, type EntryFormValues } from '../utils/entry-form-values';

type Props = {
  defaults: EntryFormValues;
  priorities: { value: string; label: string }[];
  teamSuggestions?: string[];
  peopleSuggestions?: string[];
  /** Synced entries keep their type; only manual ones can change it. */
  synced?: boolean;
  readOnly?: boolean;
  submitLabel?: string;
  onSubmit: (input: EntryInput) => Promise<unknown> | void;
  onCancel?: () => void;
  footerExtra?: React.ReactNode;
};

export function EntryForm({ defaults, priorities, teamSuggestions = [], peopleSuggestions = [], synced, readOnly, submitLabel = 'Save', onSubmit, onCancel, footerExtra }: Props) {
  const { register, handleSubmit, reset, formState } = useForm<EntryFormValues>({ resolver: zodResolver(entryFormSchema), defaultValues: defaults });
  useEffect(() => reset(defaults), [defaults, reset]);
  const { errors, isSubmitting } = formState;
  return (
    <form className="col" style={{ gap: 14 }} onSubmit={handleSubmit(async (v) => void (await onSubmit(toEntryInput(v))))} noValidate>
      <fieldset disabled={readOnly} style={{ display: 'contents' }}>
        <div className="field">
          <label htmlFor="ef-title">Title</label>
          <input id="ef-title" {...register('title')} aria-invalid={!!errors.title} />
          {errors.title && <div className="err" role="alert">{errors.title.message}</div>}
        </div>
        <div className="grid g2" style={{ gap: 12 }}>
          <div className="field">
            <label htmlFor="ef-priority">Priority</label>
            <Select id="ef-priority" {...register('priority_id')} options={priorities} placeholder="No priority" />
          </div>
          <div className="field">
            <label htmlFor="ef-type">Type</label>
            <Select id="ef-type" {...register('entry_type')} options={synced ? allTypeOptions : manualTypeOptions} disabled={readOnly || synced} />
          </div>
        </div>
        <div className="grid g2" style={{ gap: 12 }}>
          <div className="field">
            <label htmlFor="ef-date">Date</label>
            <input id="ef-date" type="date" {...register('occurred_at')} aria-invalid={!!errors.occurred_at} />
            {errors.occurred_at && <div className="err" role="alert">{errors.occurred_at.message}</div>}
          </div>
          <div className="field">
            <label htmlFor="ef-teams">Teams</label>
            <input id="ef-teams" list="ef-teams-list" placeholder="Platform, Infra" {...register('teams')} />
            <datalist id="ef-teams-list">{teamSuggestions.map((t) => <option key={t} value={t} />)}</datalist>
          </div>
        </div>
        <div className="field">
          <label htmlFor="ef-people">People</label>
          <input id="ef-people" list="ef-people-list" placeholder="Sam, Priya" {...register('collaborators')} />
          <datalist id="ef-people-list">{peopleSuggestions.map((t) => <option key={t} value={t} />)}</datalist>
        </div>
        <div className="grid g3" style={{ gap: 12 }}>
          <div className="field"><label htmlFor="ef-reach">Reach</label><Select id="ef-reach" {...register('reach')} options={reachOptions} placeholder="—" /></div>
          <div className="field"><label htmlFor="ef-cx">Complexity</label><Select id="ef-cx" {...register('complexity')} options={complexityOptions} placeholder="—" /></div>
          <div className="field"><label htmlFor="ef-role">Role</label><Select id="ef-role" {...register('role')} options={roleOptions} placeholder="—" /></div>
        </div>
        <div className="field">
          <label htmlFor="ef-desc">Notes</label>
          <textarea id="ef-desc" rows={4} {...register('description')} />
        </div>
        <div className="field">
          <label htmlFor="ef-outcome">Outcome</label>
          <input id="ef-outcome" placeholder="What changed because of this?" {...register('outcome_statement')} />
        </div>
      </fieldset>
      <div className="row between">
        <div>{footerExtra}</div>
        <div className="row">
          {onCancel && <Button onClick={onCancel}>{readOnly ? 'Close' : 'Cancel'}</Button>}
          {!readOnly && (
            <Button variant="primary" type="submit" disabled={isSubmitting}>
              {submitLabel}
            </Button>
          )}
        </div>
      </div>
    </form>
  );
}
