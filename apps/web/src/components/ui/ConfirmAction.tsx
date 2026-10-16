import { useState, type ReactNode } from 'react';
import { Dialog } from './Dialog';
import { Button } from './Button';

type Props = {
  trigger: (open: () => void) => ReactNode;
  title: string;
  message: ReactNode;
  confirmLabel?: string;
  /** When set, the user must type this exact text before confirming. */
  requireText?: string;
  onConfirm: () => void | Promise<void>;
};

export function ConfirmAction({ trigger, title, message, confirmLabel = 'Delete', requireText, onConfirm }: Props) {
  const [open, setOpen] = useState(false);
  const [typed, setTyped] = useState('');
  const [busy, setBusy] = useState(false);
  const ok = !requireText || typed === requireText;
  return (
    <>
      {trigger(() => {
        setTyped('');
        setOpen(true);
      })}
      <Dialog open={open} onOpenChange={setOpen} title={title} width={440}>
        <div className="card-b col" style={{ gap: 12 }}>
          <div>{message}</div>
          {requireText && (
            <div className="field">
              <label htmlFor="confirm-text">Type “{requireText}” to confirm</label>
              <input id="confirm-text" value={typed} onChange={(e) => setTyped(e.target.value)} autoComplete="off" />
            </div>
          )}
          <div className="row" style={{ justifyContent: 'flex-end' }}>
            <Button onClick={() => setOpen(false)}>Cancel</Button>
            <Button
              variant="primary"
              disabled={!ok || busy}
              onClick={async () => {
                setBusy(true);
                try {
                  await onConfirm();
                  setOpen(false);
                } finally {
                  setBusy(false);
                }
              }}
            >
              {confirmLabel}
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
