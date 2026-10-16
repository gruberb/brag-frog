import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Button } from '@/components/ui/Button';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { ErrorState } from '@/components/ui/EmptyState';
import { Select } from '@/components/ui/Select';
import { SkeletonRows } from '@/components/ui/Spinner';
import { useConfig } from '@/hooks/use-config';
import { toast, toastError } from '@/stores/toast';
import type { User } from '@/types/api/models';
import { useProfile, useSaveProfile } from '../api/settings';

const time = z.string().refine((v) => v === '' || /^\d{2}:\d{2}$/.test(v), 'Use HH:MM');
const schema = z.object({
  display_name: z.string().trim().min(1, 'Display name is required'),
  role: z.string(),
  team: z.string(),
  manager_name: z.string(),
  timezone: z.string(),
  week_start: z.string(),
  work_start_time: time,
  work_end_time: time,
  wants_promotion: z.boolean(),
});
type Values = z.infer<typeof schema>;

const toValues = (p: User): Values => ({
  display_name: p.display_name ?? '', role: p.role ?? '', team: p.team ?? '', manager_name: p.manager_name ?? '', timezone: p.timezone ?? '',
  week_start: p.week_start ?? 'monday', work_start_time: p.work_start_time ?? '', work_end_time: p.work_end_time ?? '', wants_promotion: p.wants_promotion,
});

function ProfileForm({ profile }: { profile: User }) {
  const cfg = useConfig();
  const save = useSaveProfile();
  const { register, handleSubmit, reset, formState: { errors, isDirty } } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: toValues(profile) });
  useEffect(() => reset(toValues(profile)), [profile, reset]);
  const tz = typeof Intl.supportedValuesOf === 'function' ? Intl.supportedValuesOf('timeZone') : [];
  const levels = cfg.data?.clg_levels.map((l) => ({ value: l.id, label: l.title })) ?? [];
  return (
    <form
      onSubmit={handleSubmit((v) =>
        // The API replaces the profile and an empty string clears a field, so unedited fields are sent back as they are.
        save.mutate(
          { ...v, display_name: v.display_name.trim(), skip_level_name: profile.skip_level_name ?? '', direct_reports: profile.direct_reports ?? '' },
          { onSuccess: () => toast('Profile saved'), onError: toastError },
        ),
      )}
      noValidate
    >
      <Card>
        <CardHeader title="Profile"><Button variant="primary" size="sm" type="submit" disabled={save.isPending || !isDirty}>Save</Button></CardHeader>
        <CardBody className="grid g2">
          <div className="field"><label htmlFor="p-name">Display name</label><input id="p-name" {...register('display_name')} aria-invalid={!!errors.display_name} />{errors.display_name && <div className="err" role="alert">{errors.display_name.message}</div>}</div>
          <div className="field"><label htmlFor="p-level">Career level</label><Select id="p-level" {...register('role')} options={levels} placeholder="Not set" /></div>
          <div className="field"><label htmlFor="p-team">Team</label><input id="p-team" {...register('team')} /></div>
          <div className="field"><label htmlFor="p-mgr">Manager</label><input id="p-mgr" {...register('manager_name')} /></div>
          <div className="field"><label htmlFor="p-tz">Timezone</label><input id="p-tz" list="tz-list" {...register('timezone')} /><datalist id="tz-list">{tz.map((z) => <option key={z} value={z} />)}</datalist></div>
          <div className="field"><label htmlFor="p-week">Week starts</label><Select id="p-week" {...register('week_start')} options={[{ value: 'monday', label: 'Monday' }, { value: 'sunday', label: 'Sunday' }]} /></div>
          <div className="field"><label htmlFor="p-ws">Work day starts</label><input id="p-ws" placeholder="09:00" {...register('work_start_time')} />{errors.work_start_time && <div className="err">{errors.work_start_time.message}</div>}</div>
          <div className="field"><label htmlFor="p-we">Work day ends</label><input id="p-we" placeholder="17:00" {...register('work_end_time')} />{errors.work_end_time && <div className="err">{errors.work_end_time.message}</div>}</div>
          <label className="small row"><input type="checkbox" {...register('wants_promotion')} /> I am going for promotion</label>
        </CardBody>
      </Card>
    </form>
  );
}

export function ProfilePanel() {
  const q = useProfile();
  if (q.isPending) return <SkeletonRows rows={3} h={60} />;
  if (q.error) return <ErrorState error={q.error} onRetry={() => q.refetch()} />;
  return <ProfileForm profile={q.data} />;
}
