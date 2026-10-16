import type { ReactNode } from 'react';
import { cn } from '@/lib/cn';

export type Tone = 'green' | 'yellow' | 'red' | 'blue' | 'purple' | 'coral' | 'gray';
export const Badge = ({ tone = 'gray', children, className }: { tone?: Tone; children: ReactNode; className?: string }) => (
  <span className={cn('badge', `b-${tone}`, className)}>{children}</span>
);
