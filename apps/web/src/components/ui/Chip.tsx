import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { cn } from '@/lib/cn';

export const Chip = ({ on, children, className }: { on?: boolean; children: ReactNode; className?: string }) => (
  <span className={cn('chip', on && 'on', className)}>{children}</span>
);

export function ChipButton({ on, className, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { on?: boolean }) {
  return <button type="button" aria-pressed={on} className={cn('chip', on && 'on', className)} {...rest} />;
}
