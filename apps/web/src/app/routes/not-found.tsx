import { Link } from 'react-router';
import { EmptyState } from '@/components/ui/EmptyState';
import { paths } from '@/config/paths';

export function Component() {
  return (
    <EmptyState title="Page not found">
      <Link className="link" to={paths.today}>Go to Today</Link>
    </EmptyState>
  );
}
