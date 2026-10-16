import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Card, CardHeader } from '@/components/ui/Card';
import { EmptyState, ErrorState } from '@/components/ui/EmptyState';
import { SkeletonRows } from '@/components/ui/Spinner';
import { useCycleContext } from '@/stores/session';
import { toastError } from '@/stores/toast';
import { useAddPerson, usePeople, useRemovePerson } from '../api/settings';

export function PeoplePanel() {
  const q = usePeople();
  const add = useAddPerson();
  const remove = useRemovePerson();
  const { readOnly } = useCycleContext();
  const [adding, setAdding] = useState(false);
  const [email, setEmail] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  return (
    <Card>
      <CardHeader title="People">{!readOnly && <Button size="sm" variant="primary" onClick={() => setAdding((a) => !a)}>+ Add</Button>}</CardHeader>
      {adding && (
        <form
          className="card-b row wrap"
          onSubmit={(e) => {
            e.preventDefault();
            if (!/^\S+@\S+\.\S+$/.test(email)) return setError('Enter a valid email');
            if (!name.trim()) return setError('Enter a name');
            setError(null);
            add.mutate({ email: email.trim(), display_name: name.trim() }, { onSuccess: () => { setEmail(''); setName(''); setAdding(false); }, onError: toastError });
          }}
        >
          <input aria-label="Email" placeholder="email@example.com" value={email} onChange={(e) => setEmail(e.target.value)} />
          <input aria-label="Name" placeholder="Display name" value={name} onChange={(e) => setName(e.target.value)} />
          <Button variant="primary" type="submit" size="sm" disabled={add.isPending}>Add person</Button>
          {error && <span className="err-text" role="alert">{error}</span>}
        </form>
      )}
      {q.isPending && <div className="card-b"><SkeletonRows rows={2} /></div>}
      {q.error && <ErrorState error={q.error} onRetry={() => q.refetch()} />}
      {q.data?.length === 0 && <EmptyState title="No aliases yet">Map email addresses to the names you want to see.</EmptyState>}
      <div className="list">
        {q.data?.map((p) => (
          <div key={p.id} className="row between">
            <span><b>{p.email}</b> · {p.display_name}{p.team ? ` · ${p.team}` : ''}</span>
            {!readOnly && <Button size="sm" variant="danger" onClick={() => remove.mutate(p.id, { onError: toastError })} aria-label={`Remove ${p.email}`}>Remove</Button>}
          </div>
        ))}
      </div>
    </Card>
  );
}
