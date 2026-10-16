import { useState, type ReactNode } from 'react';
import { QueryClientProvider } from '@tanstack/react-query';
import { ErrorBoundary } from '@/components/ui/ErrorBoundary';
import { ToastHost } from '@/components/ui/ToastHost';
import { createQueryClient } from '@/lib/react-query';

export function AppProvider({ children, client }: { children: ReactNode; client?: ReturnType<typeof createQueryClient> }) {
  const [qc] = useState(() => client ?? createQueryClient());
  return (
    <ErrorBoundary>
      <QueryClientProvider client={qc}>
        {children}
        <ToastHost />
      </QueryClientProvider>
    </ErrorBoundary>
  );
}
