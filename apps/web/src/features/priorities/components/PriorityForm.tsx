import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Button } from '@/components/ui/Button';
import { Select, type Option } from '@/components/ui/Select';
import type { PriorityInput } from '@/types/api/models';
import { priorityFormSchema, toPriorityInput, type PriorityFormValues } from '../utils/priority-form-values';

const STATUS: Option[] = [
  { value: 'active', label: 'Active' },
  { value: 'not_started', label: 'Not started' },
  { value: 'on_hold', label: 'On hold' },
  { value: 'completed', label: 'Completed' },
  { value: 'cancelled', label: 'Cancelled' },
];
const TIER: Option[] = [
  { value: 'department', label: 'Department' },
  { value: 'team', label: 'Team' },
  { value: 'individual', label: 'Individual' },
];

type Props = { defaults: PriorityFormValues; goals: Option[]; readOnly?: boolean; submitLabel: string; onSubmit: (i: PriorityInput) => Promise<unknown> | void; onCancel?: () => void };

export function PriorityForm({ defaults, goals, readOnly, submitLabel, onSubmit, onCancel }: Props) {
  const { register, handleSubmit, reset, formState } = useForm<PriorityFormValues>({ resolver: zodResolver(priorityFormSchema), defaultValues: defaults });
  useEffect(() => reset(defaults), [defaults, reset]);
  const { errors, isSubmitting } = formState;
  return (
    <form className="col" style={{ gap: 12 }} noValidate onSubmit={handleSubmit(async (v) => void (await onSubmit(toPriorityInput(v))))}>
      <fieldset disabled={readOnly} style={{ display: 'contents' }}>
        <div className="field">
          <label htmlFor="pf-title">Title</label>
          <input id="pf-title" {...register('title')} aria-invalid={!!errors.title} />
          {errors.title && <div className="err" role="alert">{errors.title.message}</div>}
        </div>
        <div className="field"><label htmlFor="pf-desc">Description</label><textarea id="pf-desc" rows={3} {...register('description')} /></div>
        <div className="grid g2" style={{ gap: 12 }}>
          <div className="field"><label htmlFor="pf-goal">Department goal</label><Select id="pf-goal" {...register('department_goal_id')} options={goals} placeholder="No goal" /></div>
          <div className="field"><label htmlFor="pf-tier">Tier</label><Select id="pf-tier" {...register('tier')} options={TIER} placeholder="—" /></div>
          <div className="field"><label htmlFor="pf-status">Status</label><Select id="pf-status" {...register('status')} options={STATUS} /></div>
          <div className="field">
            <label htmlFor="pf-due">Due date</label>
            <input id="pf-due" type="date" {...register('due_date')} />
            {errors.due_date && <div className="err" role="alert">{errors.due_date.message}</div>}
          </div>
        </div>
        <div className="grid g3" style={{ gap: 12 }}>
          <div className="field"><label htmlFor="pf-ms">Measure start</label><input id="pf-ms" inputMode="decimal" {...register('measure_start')} />{errors.measure_start && <div className="err">{errors.measure_start.message}</div>}</div>
          <div className="field"><label htmlFor="pf-mt">Measure target</label><input id="pf-mt" inputMode="decimal" {...register('measure_target')} />{errors.measure_target && <div className="err">{errors.measure_target.message}</div>}</div>
          <div className="field"><label htmlFor="pf-mc">Current</label><input id="pf-mc" inputMode="decimal" {...register('measure_current')} />{errors.measure_current && <div className="err">{errors.measure_current.message}</div>}</div>
        </div>
        <div className="field"><label htmlFor="pf-unit">Measure unit</label><input id="pf-unit" placeholder="minutes, %, tickets…" {...register('measure_type')} /></div>
      </fieldset>
      <div className="row" style={{ justifyContent: 'flex-end' }}>
        {onCancel && <Button onClick={onCancel}>Cancel</Button>}
        {!readOnly && <Button variant="primary" type="submit" disabled={isSubmitting}>{submitLabel}</Button>}
      </div>
    </form>
  );
}
