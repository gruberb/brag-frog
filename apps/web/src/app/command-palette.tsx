import * as Dialog from '@radix-ui/react-dialog';
import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { paths } from '@/config/paths';
import { useDebouncedValue } from '@/hooks/use-debounced-value';
import { useEntries } from '@/features/entries/api/entries';
import { useSync } from '@/features/integrations/api/integrations';
import { toast, toastError } from '@/stores/toast';

type Item = { id: string; label: string; hint?: string; run: () => void };

export function CommandPalette({ open, onOpenChange, cycleId }: { open: boolean; onOpenChange: (o: boolean) => void; cycleId: number | null }) {
  const nav = useNavigate();
  const sync = useSync();
  const [q, setQ] = useState('');
  const [idx, setIdx] = useState(0);
  const dq = useDebouncedValue(q.trim(), 250);
  const entries = useEntries(cycleId, { q: dq }, open && dq.length >= 2);

  const items = useMemo<Item[]>(() => {
    const go = (to: string) => () => void nav(to);
    const base: Item[] = [
      { id: 'today', label: 'Go to Today', hint: 'g t', run: go(paths.today) },
      { id: 'logbook', label: 'Go to Logbook', hint: 'g l', run: go(paths.logbook) },
      { id: 'priorities', label: 'Go to Priorities', hint: 'g p', run: go(paths.priorities) },
      { id: 'review', label: 'Go to Review', hint: 'g r', run: go(paths.review()) },
      { id: 'trends', label: 'Go to Trends', hint: 'g v', run: go(paths.trends) },
      { id: 'settings', label: 'Go to Settings', hint: 'g s', run: go(paths.settings()) },
      {
        id: 'sync',
        label: 'Sync now',
        run: () => sync.mutate({}, { onSuccess: () => toast('Sync started'), onError: toastError }),
      },
    ].filter((i) => !q || i.label.toLowerCase().includes(q.toLowerCase()));
    const found: Item[] = (entries.data?.pages[0]?.items.slice(0, 6) ?? []).map((e) => ({
      id: `e${e.id}`,
      label: e.title,
      hint: e.occurred_at,
      run: go(`${paths.logbook}?entry=${e.id}`),
    }));
    return [...base, ...found];
  }, [q, entries.data, nav, sync]);

  const choose = (i?: Item) => {
    if (!i) return;
    onOpenChange(false);
    setQ('');
    i.run();
  };

  return (
    <Dialog.Root open={open} onOpenChange={(o) => { onOpenChange(o); if (!o) setQ(''); }}>
      <Dialog.Portal>
        <Dialog.Overlay className="scrim" />
        <Dialog.Content className="card palette" aria-describedby={undefined}>
          <Dialog.Title className="sr-only">Search or jump</Dialog.Title>
          <input
            autoFocus
            aria-label="Search or jump"
            placeholder="Search entries or jump to…"
            value={q}
            onChange={(e) => { setQ(e.target.value); setIdx(0); }}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown') { e.preventDefault(); setIdx((i) => Math.min(i + 1, items.length - 1)); }
              if (e.key === 'ArrowUp') { e.preventDefault(); setIdx((i) => Math.max(i - 1, 0)); }
              if (e.key === 'Enter') { e.preventDefault(); choose(items[idx]); }
            }}
            style={{ width: '100%', border: 0, borderBottom: '1px solid var(--color-border)', borderRadius: 0, padding: '14px 16px' }}
          />
          <div role="listbox" style={{ maxHeight: 340, overflow: 'auto', padding: 4 }}>
            {items.map((i, n) => (
              <button
                key={i.id}
                role="option"
                aria-selected={n === idx}
                type="button"
                className="btn ghost"
                style={{ width: '100%', justifyContent: 'space-between', fontWeight: 400, background: n === idx ? 'var(--color-active-bg)' : undefined }}
                onMouseEnter={() => setIdx(n)}
                onClick={() => choose(i)}
              >
                <span className="clamp">{i.label}</span>
                {i.hint && <span className="mono muted">{i.hint}</span>}
              </button>
            ))}
            {items.length === 0 && <div className="empty small">Nothing found</div>}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
