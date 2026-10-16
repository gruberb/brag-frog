import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import type { ReactNode } from 'react';

export type MenuItem = { label: string; onSelect: () => void; disabled?: boolean };

const content: React.CSSProperties = {
  background: 'var(--color-surface)',
  border: '1px solid var(--color-border)',
  borderRadius: 'var(--radius-md)',
  boxShadow: 'var(--shadow-md)',
  padding: 4,
  minWidth: 160,
  zIndex: 60,
};

export function Menu({ trigger, items }: { trigger: ReactNode; items: MenuItem[] }) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>{trigger}</DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content align="end" sideOffset={4} style={content}>
          {items.map((i) => (
            <DropdownMenu.Item
              key={i.label}
              disabled={i.disabled}
              onSelect={i.onSelect}
              style={{ padding: '7px 12px', borderRadius: 6, cursor: 'pointer', fontSize: 14, outline: 'none' }}
              className="menu-item"
            >
              {i.label}
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
