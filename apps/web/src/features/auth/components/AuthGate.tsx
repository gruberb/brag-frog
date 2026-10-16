import type { ReactNode } from 'react';
import { ApiError } from '@/lib/api-client';
import { SkeletonRows } from '@/components/ui/Spinner';
import { ErrorState } from '@/components/ui/EmptyState';
import { useMe } from '../api/auth';

/** Blocks children until /me resolves. A 401 is handled globally (redirect to /signin), so only other errors render here. */
export function AuthGate({ children }: { children: ReactNode }) {
  const me = useMe();
  if (me.isPending) return <div className="main"><SkeletonRows rows={4} /></div>;
  if (me.error) {
    if (me.error instanceof ApiError && me.error.status === 401) return null;
    return <div className="main"><ErrorState error={me.error} onRetry={() => me.refetch()} /></div>;
  }
  return <>{children}</>;
}
