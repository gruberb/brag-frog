import type { HTMLAttributes, ReactNode } from 'react';
import { cn } from '@/lib/cn';

export const Card = ({ className, ...rest }: HTMLAttributes<HTMLDivElement>) => <div className={cn('card', className)} {...rest} />;
export const CardHeader = ({ title, children }: { title: ReactNode; children?: ReactNode }) => (
  <div className="card-h">
    <h2>{title}</h2>
    {children}
  </div>
);
export const CardBody = ({ className, ...rest }: HTMLAttributes<HTMLDivElement>) => <div className={cn('card-b', className)} {...rest} />;
