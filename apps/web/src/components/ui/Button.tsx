import type { ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/cn';

type Props = ButtonHTMLAttributes<HTMLButtonElement> & { variant?: 'default' | 'primary' | 'ghost' | 'danger'; size?: 'md' | 'sm' };

export function Button({ variant = 'default', size = 'md', className, type = 'button', ...rest }: Props) {
  return <button type={type} className={cn('btn', variant !== 'default' && variant, size === 'sm' && 'sm', className)} {...rest} />;
}
