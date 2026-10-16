import * as RadixTabs from '@radix-ui/react-tabs';
import type { ReactNode } from 'react';

export type TabItem = { value: string; label: ReactNode };

/** Controlled tab strip; panels are rendered by the caller, so URL state stays in the route. */
export function TabList({ items, value, onChange, className = 'tabs' }: { items: TabItem[]; value: string; onChange: (v: string) => void; className?: string }) {
  return (
    <RadixTabs.Root value={value} onValueChange={onChange}>
      <RadixTabs.List className={className}>
        {items.map((t) => (
          <RadixTabs.Trigger key={t.value} value={t.value}>
            {t.label}
          </RadixTabs.Trigger>
        ))}
      </RadixTabs.List>
    </RadixTabs.Root>
  );
}
