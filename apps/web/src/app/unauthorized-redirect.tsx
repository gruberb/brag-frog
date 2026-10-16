import { useEffect } from 'react';
import type { createBrowserRouter } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { setUnauthorizedHandler } from '@/lib/api-client';
import { paths } from '@/config/paths';

/** 401 anywhere clears cached data (it belongs to the previous session) and goes to the sign-in page. */
export function UnauthorizedRedirect({ router }: { router: ReturnType<typeof createBrowserRouter> }) {
  const qc = useQueryClient();
  useEffect(() => {
    setUnauthorizedHandler(() => {
      qc.clear();
      if (router.state.location.pathname !== paths.signin) void router.navigate(paths.signin, { replace: true });
    });
    return () => setUnauthorizedHandler(null);
  }, [qc, router]);
  return null;
}
