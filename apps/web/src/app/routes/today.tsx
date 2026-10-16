import { QuickComposer } from '@/features/entries/components/Composer';
import { TodayPage } from '@/features/today/components/TodayPage';

export function Component() {
  return <TodayPage composer={<QuickComposer />} />;
}
