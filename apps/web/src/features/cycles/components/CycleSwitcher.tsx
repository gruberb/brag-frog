import * as Popover from '@radix-ui/react-popover';
import { useState } from 'react';
import type { Cycle } from '@/types/api/models';

type Props = { cycle: Cycle | null; cycles: Cycle[]; activeId: number | null; onSelect: (id: number | null) => void };

export function CycleSwitcher({ cycle, cycles, activeId, onSelect }: Props) {
  const [open, setOpen] = useState(false);
  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger asChild>
        <button type="button" className="cycle-switch" title="Switch performance cycle" aria-label="Switch performance cycle">
          <span className="l">Cycle</span>
          <b>
            <span>{cycle ? `${cycle.name}${cycle.id !== activeId ? ' (past)' : ''}` : 'No cycle'}</span>
            <span aria-hidden>⌄</span>
          </b>
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content className="card" sideOffset={4} align="start" style={{ padding: 4, width: 216, zIndex: 60, boxShadow: 'var(--shadow-md)' }}>
          {cycles.map((c) => (
            <button
              key={c.id}
              type="button"
              className="btn ghost"
              style={{ width: '100%', justifyContent: 'space-between', fontWeight: c.id === cycle?.id ? 600 : 400 }}
              onClick={() => {
                onSelect(c.id === activeId ? null : c.id);
                setOpen(false);
              }}
            >
              <span>{c.name}</span>
              {c.id === activeId && <span className="badge b-green">Active</span>}
            </button>
          ))}
          {cycles.length === 0 && <div className="empty small">No cycles yet</div>}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
