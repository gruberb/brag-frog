import * as Dialog from '@radix-ui/react-dialog';
import type { ReactNode } from 'react';
import { Button } from './Button';

type Props = {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  title: ReactNode;
  subtitle?: ReactNode;
  children: ReactNode;
};

/** Right-hand panel. Closed by default everywhere so lists stay readable. */
export function Drawer({ open, onOpenChange, title, subtitle, children }: Props) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="scrim" />
        <Dialog.Content className="drawer" aria-describedby={undefined}>
          <div className="drawer-h">
            <div>
              <Dialog.Title asChild>
                <h3>{title}</h3>
              </Dialog.Title>
              {subtitle && <span className="small muted">{subtitle}</span>}
            </div>
            <Dialog.Close asChild>
              <Button variant="ghost" size="sm" aria-label="Close">
                ✕
              </Button>
            </Dialog.Close>
          </div>
          <div className="drawer-b">{children}</div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
