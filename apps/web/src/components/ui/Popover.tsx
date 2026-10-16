import * as RadixPopover from '@radix-ui/react-popover';
import type { ReactNode } from 'react';

export function Popover({ trigger, children }: { trigger: ReactNode; children: ReactNode }) {
  return (
    <RadixPopover.Root>
      <RadixPopover.Trigger asChild>{trigger}</RadixPopover.Trigger>
      <RadixPopover.Portal>
        <RadixPopover.Content
          align="end"
          sideOffset={6}
          className="card"
          style={{ padding: 12, width: 300, zIndex: 60, boxShadow: 'var(--shadow-md)' }}
        >
          {children}
        </RadixPopover.Content>
      </RadixPopover.Portal>
    </RadixPopover.Root>
  );
}
