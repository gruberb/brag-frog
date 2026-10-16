import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { Drawer } from '@/components/ui/Drawer';
import { useConfig } from '@/hooks/use-config';
import { useCycleContext } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { Integration } from '@/types/api/models';
import { relativeTime } from '@/utils/dates';
import { useResetIntegration, useRestoreExcluded, useSaveIntegration, useSync, useSyncStatus, useTestIntegration } from '../api/integrations';

type Props = { service: string | null; integration: Integration | null; onClose: () => void };

function Form({ integration, onClose }: { integration: Integration; onClose: () => void }) {
  const cfg = useConfig();
  const { readOnly } = useCycleContext();
  const meta = cfg.data?.services.find((s) => s.service === integration.service);
  const status = useSyncStatus();
  const save = useSaveIntegration();
  const test = useTestIntegration();
  const reset = useResetIntegration();
  const restore = useRestoreExcluded();
  const sync = useSync();
  const [token, setToken] = useState('');
  const [fields, setFields] = useState<Record<string, string>>({
    base_url: integration.config.base_url ?? meta?.default_base_url ?? '',
    email: integration.config.email ?? '',
    orgs: integration.config.orgs ?? meta?.default_orgs ?? '',
  });
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);
  const [resetting, setResetting] = useState(false);
  const [alsoDelete, setAlsoDelete] = useState(false);
  const f = (k: string) => ({ value: fields[k], onChange: (e: { target: { value: string } }) => setFields((x) => ({ ...x, [k]: e.target.value })) });
  const logs = status.data?.recent_logs.filter((l) => l.service === integration.service) ?? [];

  return (
    <form
      className="col"
      style={{ gap: 14 }}
      onSubmit={(e) => {
        e.preventDefault();
        const config = Object.fromEntries(Object.entries(fields).filter(([k, v]) => v !== '' && ((k === 'base_url' && meta?.base_url_placeholder) || (k === 'email' && meta?.email_placeholder) || (k === 'orgs' && meta?.org_placeholder))));
        save.mutate({ service: integration.service, input: { config, ...(token ? { token } : {}) } }, { onSuccess: () => { setToken(''); toast('Saved'); }, onError: toastError });
      }}
    >
      {meta?.note && <div className="note">{meta.note}</div>}
      <fieldset disabled={readOnly} style={{ display: 'contents' }}>
        {meta?.base_url_placeholder && <div className="field"><label htmlFor="i-url">Base URL</label><input id="i-url" placeholder={meta.base_url_placeholder} {...f('base_url')} /></div>}
        {meta?.email_placeholder && <div className="field"><label htmlFor="i-email">Email</label><input id="i-email" type="email" placeholder={meta.email_placeholder} {...f('email')} /></div>}
        {meta?.org_placeholder && <div className="field"><label htmlFor="i-orgs">Organizations</label><input id="i-orgs" placeholder={meta.org_placeholder} {...f('orgs')} /></div>}
        <div className="field">
          <label htmlFor="i-token">Token</label>
          <input id="i-token" type="password" autoComplete="off" placeholder={integration.has_token ? 'Saved. Enter a new token to replace it' : 'Paste a token'} value={token} onChange={(e) => setToken(e.target.value)} />
          {meta?.token_url && <a className="link small" href={meta.token_url} target="_blank" rel="noreferrer">Create a token ↗</a>}
        </div>
      </fieldset>
      <div className="row wrap">
        {!readOnly && <Button variant="primary" type="submit" disabled={save.isPending}>Save</Button>}
        <Button disabled={test.isPending || !integration.has_token} onClick={() => test.mutate(integration.service, { onSuccess: setResult, onError: toastError })}>Test connection</Button>
        {!readOnly && <Button disabled={integration.status === 'not_connected'} onClick={() => sync.mutate({ service: integration.service }, { onSuccess: () => toast('Sync started'), onError: toastError })}>Sync now</Button>}
      </div>
      {result && <div role="status" className={result.ok ? 'small' : 'err-text'}>{result.message}</div>}

      {integration.excluded.length > 0 && (
        <div className="col">
          <h3>Excluded items</h3>
          {integration.excluded.map((x) => (
            <div key={x.id} className="row between">
              <span>{x.label}</span>
              {!readOnly && <Button size="sm" onClick={() => restore.mutate({ service: integration.service, id: x.id }, { onError: toastError })} aria-label={`Restore ${x.label}`}>Restore</Button>}
            </div>
          ))}
        </div>
      )}

      <div className="col">
        <h3>Activity</h3>
        {logs.length === 0 && <span className="small muted">No recent activity</span>}
        {logs.map((l) => <div key={l.id} className="small"><b>{l.status}</b> · {l.message ?? ''} <span className="muted">{relativeTime(l.created_at)}</span></div>)}
      </div>

      {!readOnly && integration.status !== 'not_connected' && (
        <div>
          <Button variant="danger" onClick={() => setResetting(true)}>Reset integration</Button>
          <Dialog open={resetting} onOpenChange={setResetting} title={`Reset ${meta?.label ?? integration.service}`} width={440}>
            <div className="card-b col">
              <p style={{ margin: 0 }}>This removes the saved token and settings.</p>
              <label className="small row"><input type="checkbox" checked={alsoDelete} onChange={(e) => setAlsoDelete(e.target.checked)} /> Also delete the {integration.entry_count} synced entries</label>
              <div className="row" style={{ justifyContent: 'flex-end' }}>
                <Button onClick={() => setResetting(false)}>Cancel</Button>
                <Button variant="primary" onClick={() => reset.mutate({ service: integration.service, deleteEntries: alsoDelete }, { onSuccess: () => { toast('Integration reset'); setResetting(false); onClose(); }, onError: toastError })}>Reset</Button>
              </div>
            </div>
          </Dialog>
        </div>
      )}
    </form>
  );
}

export function ConfigureDrawer({ service, integration, onClose }: Props) {
  const cfg = useConfig();
  const label = cfg.data?.services.find((s) => s.service === service)?.label ?? service ?? '';
  return (
    <Drawer open={service != null} onOpenChange={(o) => !o && onClose()} title={label} subtitle="Integration settings">
      {integration && <Form key={integration.service} integration={integration} onClose={onClose} />}
    </Drawer>
  );
}
