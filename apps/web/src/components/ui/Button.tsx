import type { ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/cn';

/**
 * Semantic variants, one visual style per intent so the same action looks the same everywhere:
 * primary (add/submit), transfer (import/export/download), action (completes or activates something),
 * danger (delete/remove), toggle (switches a UI mode, shows its state), ghost (chrome such as close).
 */
type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'default' | 'primary' | 'ghost' | 'danger' | 'transfer' | 'action' | 'toggle';
  size?: 'md' | 'sm';
  /** Decorative leading glyph, hidden from assistive tech so the accessible name stays the label. */
  icon?: string;
};

export function Button({ variant = 'default', size = 'md', icon, className, type = 'button', children, ...rest }: Props) {
  return (
    <button type={type} className={cn('btn', variant !== 'default' && variant, size === 'sm' && 'sm', className)} {...rest}>
      {icon && <span aria-hidden="true">{icon}</span>}
      {children}
    </button>
  );
}
