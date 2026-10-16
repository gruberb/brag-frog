import { createBrowserRouter, Navigate, type RouteObject } from 'react-router';
import { paths } from '@/config/paths';

/** Each route module exports `Component`; `lazy()` keeps heavy views (editor, export, charts) out of the first load. */
const lazyRoute = (load: () => Promise<{ Component: React.ComponentType }>) => ({ lazy: async () => ({ Component: (await load()).Component }) });

export const routes: RouteObject[] = [
  { path: paths.signin, ...lazyRoute(() => import('./routes/signin')) },
  { path: paths.privacy, ...lazyRoute(() => import('./routes/legal').then((m) => ({ Component: m.Privacy }))) },
  { path: paths.terms, ...lazyRoute(() => import('./routes/legal').then((m) => ({ Component: m.Terms }))) },
  {
    ...lazyRoute(() => import('./routes/authenticated-layout')),
    children: [
      { index: true, element: <Navigate to={paths.today} replace /> },
      { path: 'today', ...lazyRoute(() => import('./routes/today')) },
      { path: 'logbook', ...lazyRoute(() => import('./routes/logbook')) },
      { path: 'priorities', ...lazyRoute(() => import('./routes/priorities')) },
      { path: 'review/:cycleId?/:section?', ...lazyRoute(() => import('./routes/review')) },
      { path: 'trends', ...lazyRoute(() => import('./routes/trends')) },
      { path: 'settings/:tab?', ...lazyRoute(() => import('./routes/settings')) },
      { path: '*', ...lazyRoute(() => import('./routes/not-found')) },
    ],
  },
];

export const router = createBrowserRouter(routes);
