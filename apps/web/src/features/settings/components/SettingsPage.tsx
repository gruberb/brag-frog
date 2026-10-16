import type { ReactNode } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router';
import { TabList } from '@/components/ui/Tabs';
import { paths, settingsTabs, type SettingsTab } from '@/config/paths';
import { AiPanel } from './AiPanel';
import { PeoplePanel } from './PeoplePanel';
import { ProfilePanel } from './ProfilePanel';

const LABELS: Record<SettingsTab, string> = { profile: 'Profile', cycles: 'Cycles', people: 'People', integrations: 'Integrations', ai: 'AI' };

/** Cycles and Integrations panels belong to their own features and are passed in by the route. */
export function SettingsPage({ cycles, integrations }: { cycles: ReactNode; integrations: ReactNode }) {
  const { tab } = useParams();
  const nav = useNavigate();
  const [sp] = useSearchParams();
  const active: SettingsTab = (settingsTabs as readonly string[]).includes(tab ?? '') ? (tab as SettingsTab) : 'profile';
  const panels: Record<SettingsTab, ReactNode> = { profile: <ProfilePanel />, cycles, people: <PeoplePanel />, integrations, ai: <AiPanel /> };
  return (
    <>
      <div className="page-head"><div><h1>Settings</h1><p>Profile, performance cycles, people, integrations and AI.</p></div></div>
      <TabList value={active} onChange={(t) => void nav({ pathname: paths.settings(t), search: sp.get('cycle') ? `?cycle=${sp.get('cycle')}` : '' })} items={settingsTabs.map((t) => ({ value: t, label: LABELS[t] }))} />
      <section aria-label={LABELS[active]}>{panels[active]}</section>
    </>
  );
}
