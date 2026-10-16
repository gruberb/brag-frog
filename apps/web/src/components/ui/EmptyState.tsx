import type { ReactNode } from 'react';
import { Button } from './Button';

export const EmptyState = ({ title, children }: { title: string; children?: ReactNode }) => (
  <div className="empty">
    <b style={{ display: 'block', color: 'var(--color-text)', fontWeight: 500 }}>{title}</b>
    {children && <div style={{ marginTop: 4 }}>{children}</div>}
  </div>
);

export const ErrorState = ({ error, onRetry }: { error: unknown; onRetry?: () => void }) => (
  <div className="empty" role="alert">
    <b style={{ display: 'block', color: '#b91c1c', fontWeight: 500 }}>Could not load this</b>
    <div style={{ margin: '4px 0 12px' }}>{error instanceof Error ? error.message : 'Unknown error'}</div>
    {onRetry && <Button onClick={onRetry}>Retry</Button>}
  </div>
);
