import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { toast, toastError } from '@/stores/toast';
import { useCreateCycle } from '../api/cycles';

const date = z.string().regex(/^\d{4}-\d{2}-\d{2}$/, 'Pick a date');
const schema = z
  .object({ name: z.string().trim().min(1, 'Name is required').max(80), start_date: date, end_date: date, activate: z.boolean() })
  .refine((v) => v.end_date >= v.start_date, { path: ['end_date'], message: 'End must be on or after the start' });
type Values = z.infer<typeof schema>;

export function CycleDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const create = useCreateCycle();
  const year = new Date().getFullYear();
  const { register, handleSubmit, reset, formState: { errors } } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { name: '', start_date: `${year}-01-01`, end_date: `${year}-06-30`, activate: true } });
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="New cycle" width={460}>
      <form
        className="card-b col"
        noValidate
        onSubmit={handleSubmit((v) =>
          create.mutate(v, { onSuccess: () => { toast('Cycle created'); reset(); onOpenChange(false); }, onError: toastError }),
        )}
      >
        <div className="field"><label htmlFor="c-name">Name</label><input id="c-name" placeholder={`H1 ${year}`} {...register('name')} />{errors.name && <div className="err" role="alert">{errors.name.message}</div>}</div>
        <div className="grid g2" style={{ gap: 12 }}>
          <div className="field"><label htmlFor="c-start">Start</label><input id="c-start" type="date" {...register('start_date')} /></div>
          <div className="field"><label htmlFor="c-end">End</label><input id="c-end" type="date" {...register('end_date')} />{errors.end_date && <div className="err" role="alert">{errors.end_date.message}</div>}</div>
        </div>
        <label className="small row"><input type="checkbox" {...register('activate')} /> Make this the active cycle</label>
        <div className="row" style={{ justifyContent: 'flex-end' }}>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" type="submit" disabled={create.isPending}>Create cycle</Button>
        </div>
      </form>
    </Dialog>
  );
}
