import { useCallback, useMemo, useState } from 'react';
import { Outlet, useLocation, useNavigate, useSearchParams } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { AppShell } from '@/components/layout/AppShell';
import { ErrorBoundary } from '@/components/ui/ErrorBoundary';
import { paths } from '@/config/paths';
import { AuthGate } from '@/features/auth/components/AuthGate';
import { useMe } from '@/features/auth/api/auth';
import { useCycles } from '@/features/cycles/api/cycles';
import { CycleSwitcher } from '@/features/cycles/components/CycleSwitcher';
import { ReadOnlyBanner } from '@/features/cycles/components/ReadOnlyBanner';
import { SyncIndicator } from '@/features/integrations/components/SyncIndicator';
import { FOCUS_COMPOSER_EVENT } from '@/features/entries/components/Composer';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';
import { CycleContext, SessionContext, type CycleContextValue } from '@/stores/session';
import { CommandPalette } from '../command-palette';

const CYCLE_PARAM = 'cycle';

function Authenticated() {
  const me = useMe().data!;
  const cyclesQ = useCycles();
  const [sp, setSp] = useSearchParams();
  const nav = useNavigate();
  const loc = useLocation();
  const qc = useQueryClient();
  const [palette, setPalette] = useState(false);

  const cycles = useMemo(() => cyclesQ.data ?? (me.active_cycle ? [me.active_cycle] : []), [cyclesQ.data, me.active_cycle]);
  const active = cycles.find((c) => c.is_active) ?? me.active_cycle;
  const selectedId = Number(sp.get(CYCLE_PARAM)) || null;
  const cycle = cycles.find((c) => c.id === selectedId) ?? active ?? null;
  const readOnly = !!cycle && cycle.id !== active?.id;

  const selectCycle = useCallback(
    (id: number | null) =>
      setSp((prev) => {
        const next = new URLSearchParams(prev);
        if (id) next.set(CYCLE_PARAM, String(id));
        else next.delete(CYCLE_PARAM);
        return next;
      }),
    [setSp],
  );

  const ctx = useMemo<CycleContextValue>(() => ({ cycle, activeCycle: active ?? null, cycles, readOnly, selectCycle }), [cycle, active, cycles, readOnly, selectCycle]);

  useKeyboardShortcuts({
    onPalette: () => setPalette(true),
    onComposer: () => {
      if (loc.pathname !== paths.today) void nav(paths.today);
      setTimeout(() => window.dispatchEvent(new Event(FOCUS_COMPOSER_EVENT)), 50);
    },
    chords: {
      t: () => void nav(paths.today),
      l: () => void nav(paths.logbook),
      p: () => void nav(paths.priorities),
      r: () => void nav(paths.review()),
      v: () => void nav(paths.trends),
      d: () => void nav(paths.today),
      s: () => void nav(paths.settings()),
    },
  });

  const search = selectedId ? `?${CYCLE_PARAM}=${selectedId}` : '';

  return (
    <SessionContext.Provider value={me}>
      <CycleContext.Provider value={ctx}>
        <AppShell
          userEmail={me.user.email}
          search={search}
          onOpenPalette={() => setPalette(true)}
          cycleSwitcher={<CycleSwitcher cycle={cycle} cycles={cycles} activeId={active?.id ?? null} onSelect={(id) => { selectCycle(id); void qc.invalidateQueries(); }} />}
          syncIndicator={<SyncIndicator />}
          banner={readOnly && cycle ? <ReadOnlyBanner cycle={cycle} onBack={() => selectCycle(null)} /> : null}
        >
          <ErrorBoundary key={loc.pathname}>
            <Outlet />
          </ErrorBoundary>
        </AppShell>
        <CommandPalette open={palette} onOpenChange={setPalette} cycleId={cycle?.id ?? null} />
      </CycleContext.Provider>
    </SessionContext.Provider>
  );
}

export function Component() {
  return (
    <AuthGate>
      <Authenticated />
    </AuthGate>
  );
}
