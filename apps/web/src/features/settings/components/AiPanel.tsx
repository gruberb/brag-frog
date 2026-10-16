import { useState } from 'react';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { useSession } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import { useRemoveAiKey, useSaveAiKey, useTestAiKey } from '../api/settings';

export function AiPanel() {
  const { has_ai } = useSession();
  const save = useSaveAiKey();
  const test = useTestAiKey();
  const remove = useRemoveAiKey();
  const [key, setKey] = useState('');
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);
  return (
    <Card>
      <CardHeader title="AI">{has_ai ? <Badge tone="green">Key saved</Badge> : <Badge>Not set</Badge>}</CardHeader>
      <CardBody className="col">
        <div className="field">
          <label htmlFor="ai-key">Anthropic API key</label>
          <input id="ai-key" type="password" autoComplete="off" placeholder={has_ai ? 'Saved. Enter a new key to replace it' : 'Stored encrypted, only used for your drafts'} value={key} onChange={(e) => setKey(e.target.value)} />
        </div>
        <div className="row wrap">
          <Button variant="primary" size="sm" disabled={!key.trim() || save.isPending} onClick={() => save.mutate(key.trim(), { onSuccess: () => { setKey(''); toast('Key saved'); }, onError: toastError })}>Save key</Button>
          <Button size="sm" disabled={!has_ai || test.isPending} onClick={() => test.mutate(undefined, { onSuccess: setResult, onError: toastError })}>Test</Button>
          {has_ai && <Button size="sm" variant="ghost" onClick={() => remove.mutate(undefined, { onSuccess: () => toast('Key removed'), onError: toastError })}>Remove key</Button>}
        </div>
        {result && <div role="status" className={result.ok ? 'small' : 'err-text'}>{result.message}</div>}
      </CardBody>
    </Card>
  );
}
