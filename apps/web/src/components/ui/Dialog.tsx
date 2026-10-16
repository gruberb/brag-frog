import * as RadixDialog from '@radix-ui/react-dialog';
import type { ReactNode } from 'react';
import { Button } from './Button';

type Props = { open: boolean; onOpenChange: (o: boolean) => void; title: string; children: ReactNode; width?: number };

export function Dialog({ open, onOpenChange, title, children, width }: Props) {
  return (
    <RadixDialog.Root open={open} onOpenChange={onOpenChange}>
      <RadixDialog.Portal>
        <RadixDialog.Overlay className="scrim" />
        <RadixDialog.Content className="card dlg" style={width ? { width } : undefined} aria-describedby={undefined}>
          <div className="card-h">
            <RadixDialog.Title asChild>
              <h2>{title}</h2>
            </RadixDialog.Title>
            <RadixDialog.Close asChild>
              <Button variant="ghost" size="sm" aria-label="Close">
                ✕
              </Button>
            </RadixDialog.Close>
          </div>
          {children}
        </RadixDialog.Content>
      </RadixDialog.Portal>
    </RadixDialog.Root>
  );
}
