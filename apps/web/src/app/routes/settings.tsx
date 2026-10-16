import { CyclesPanel } from '@/features/cycles/components/CyclesPanel';
import { IntegrationsPanel } from '@/features/integrations/components/IntegrationsPanel';
import { SettingsPage } from '@/features/settings/components/SettingsPage';

/** Settings owns the tabs; cycles and integrations are separate features composed in here. */
export function Component() {
  return <SettingsPage cycles={<CyclesPanel />} integrations={<IntegrationsPanel />} />;
}
