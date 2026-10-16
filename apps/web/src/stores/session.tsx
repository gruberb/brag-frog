import { createContext, useContext } from 'react';
import type { Cycle, Me } from '@/types/api/models';

/** Signed-in user, provided once by the app layer after the auth gate. */
export const SessionContext = createContext<Me | null>(null);
export function useSession(): Me {
  const me = useContext(SessionContext);
  if (!me) throw new Error('useSession must be used inside the authenticated layout');
  return me;
}

export type CycleContextValue = {
  /** Cycle being viewed: `?cycle=ID` or the active one. Null when the user has no cycle yet. */
  cycle: Cycle | null;
  activeCycle: Cycle | null;
  cycles: Cycle[];
  /** Past cycles are read-only; mutations are hidden. */
  readOnly: boolean;
  selectCycle: (id: number | null) => void;
};

export const CycleContext = createContext<CycleContextValue | null>(null);
export function useCycleContext(): CycleContextValue {
  const v = useContext(CycleContext);
  if (!v) throw new Error('useCycleContext must be used inside the authenticated layout');
  return v;
}
