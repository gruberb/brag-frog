import type { ReactElement } from 'react';
import { render } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { createMemoryRouter, MemoryRouter, RouterProvider } from 'react-router';
import { createQueryClient } from '@/lib/react-query';
import { CycleContext, SessionContext, type CycleContextValue } from '@/stores/session';
import { ToastHost } from '@/components/ui/ToastHost';
import { routes } from '@/app/router';
import { UnauthorizedRedirect } from '@/app/unauthorized-redirect';
import type { Cycle, Me } from '@/types/api/models';

export const activeCycle: Cycle = { id: 2, name: 'H2 2026', start_date: '2026-07-01', end_date: '2026-12-31', is_active: true, entry_count: 90, priority_count: 4 };
export const pastCycle: Cycle = { id: 1, name: 'H1 2026', start_date: '2026-01-01', end_date: '2026-06-30', is_active: false, entry_count: 6, priority_count: 0 };

export const testMe: Me = { user: { id: 1, email: 'bgruber@example.com', name: 'Bastian Gruber', display_name: 'Bastian', avatar_url: null, role: 'senior' }, active_cycle: activeCycle, has_ai: true, public_only: false };

type Opts = { route?: string; me?: Partial<Me>; cycle?: Cycle; readOnly?: boolean };

/** Renders a feature component with query, router, session and cycle context, as the authenticated layout would. */
export function renderWithProviders(ui: ReactElement, { route = '/', me, cycle = activeCycle, readOnly = false }: Opts = {}) {
  const qc = createQueryClient();
  qc.setDefaultOptions({ queries: { retry: false, staleTime: 0, gcTime: Infinity } });
  const cycleCtx: CycleContextValue = { cycle, activeCycle, cycles: [activeCycle, pastCycle], readOnly, selectCycle: () => {} };
  const result = render(
    <QueryClientProvider client={qc}>
      <SessionContext.Provider value={{ ...testMe, ...me }}>
        <CycleContext.Provider value={cycleCtx}>
          <MemoryRouter initialEntries={[route]}>{ui}</MemoryRouter>
          <ToastHost />
        </CycleContext.Provider>
      </SessionContext.Provider>
    </QueryClientProvider>,
  );
  return { ...result, qc };
}

/** Renders the real route tree (auth gate, shell, lazy routes). */
export function renderApp(route = '/today') {
  const qc = createQueryClient();
  qc.setDefaultOptions({ queries: { retry: false, gcTime: Infinity } });
  const router = createMemoryRouter(routes, { initialEntries: [route] });
  const result = render(
    <QueryClientProvider client={qc}>
      <UnauthorizedRedirect router={router} />
      <RouterProvider router={router} />
      <ToastHost />
    </QueryClientProvider>,
  );
  return { ...result, router, qc };
}
