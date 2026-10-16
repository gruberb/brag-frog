import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { Drawer } from '@/components/ui/Drawer';
import { useCycleContext } from '@/stores/session';
import { toast, toastError } from '@/stores/toast';
import type { Integration, IntegrationInput } from '@/types/api/models';
import { relativeTime } from '@/utils/dates';
import { useResetIntegration, useRestoreExcluded, useSaveIntegration, useSync, useSyncStatus, useTestIntegration } from '../api/integrations';
import { emptyInput, integrationState } from '../utils/integration-status';

type Props = { service: string | null; integration: Integration | null; onClose: () => void };
type Values = Record<string, string | boolean>;

/** Field names are the keys of `IntegrationInput`; text fields left empty are sent as null. */
function buildInput(integration: Integration, values: Values, token: string): IntegrationInput {
  const input = emptyInput() as Record<string, unknown>;
  for (const f of integration.fields) {
    const v = values[f.name];
    input[f.name] = f.field_type === 'checkbox' ? v === true : typeof v === 'string' && v !== '' ? v : null;
  }
  input.token = token || null;
  return input as unknown as IntegrationInput;
}

function Form({ integration, onClose }: { integration: Integration; onClose: () => void }) {
  const { readOnly } = useCycleContext();
  const status = useSyncStatus();
  const save = useSaveIntegration();
  const test = useTestIntegration();
  const reset = useResetIntegration();
  const restore = useRestoreExcluded();
  const sync = useSync();
  const [token, setToken] = useState('');
  const [values, setValues] = useState<Values>(() => Object.fromEntries(integration.fields.map((f) => [f.name, f.value])));
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);
  const [resetting, setResetting] = useState(false);
  const [alsoDelete, setAlsoDelete] = useState(false);
  const logs = status.data?.recent_logs.filter((l) => l.service === integration.service) ?? [];
  const excluded = [...integration.excluded_files, ...integration.excluded_events];
  const state = integrationState(integration);
  const needsToken = !integration.oauth_connect;

  return (
    <form
      className="col"
      style={{ gap: 14 }}
      onSubmit={(e) => {
        e.preventDefault();
        // Saving tests the connection on the server; a failed test comes back as an error toast.
        save.mutate(
          { service: integration.service, input: buildInput(integration, values, token) },
          {
            onSuccess: (saved) => {
              setToken('');
              toast('Saved');
              // The first sync starts explicitly, as the API does not run it on save.
              if (!integration.is_enabled && saved.is_enabled) sync.mutate({ service: integration.service });
            },
            onError: toastError,
          },
        );
      }}
    >
      {integration.note && <div className="note">{integration.note}</div>}
      {integration.oauth_connect && <div className="small muted">Uses your Google sign-in; no token needed.</div>}
      <fieldset disabled={readOnly} style={{ display: 'contents' }}>
        {integration.fields.map((f) =>
          f.field_type === 'checkbox' ? (
            <label key={f.name} className="small row"><input type="checkbox" checked={values[f.name] === true} onChange={(e) => setValues((x) => ({ ...x, [f.name]: e.target.checked }))} /> {f.label}</label>
          ) : (
            <div key={f.name} className="field">
              <label htmlFor={`i-${f.name}`}>{f.label}</label>
              <input id={`i-${f.name}`} placeholder={f.placeholder ?? undefined} value={String(values[f.name] ?? '')} onChange={(e) => setValues((x) => ({ ...x, [f.name]: e.target.value }))} />
            </div>
          ),
        )}
        {needsToken && (
          <div className="field">
            <label htmlFor="i-token">Token</label>
            <input id="i-token" type="password" autoComplete="off" placeholder={integration.has_token ? 'Saved. Enter a new token to replace it' : 'Paste a token'} value={token} onChange={(e) => setToken(e.target.value)} />
            {integration.token_url && <a className="link small" href={integration.token_url} target="_blank" rel="noreferrer">Create a token ↗</a>}
          </div>
        )}
      </fieldset>
      <div className="row wrap">
        {!readOnly && <Button variant="primary" type="submit" disabled={save.isPending}>{integration.is_enabled ? 'Save' : 'Connect'}</Button>}
        <Button disabled={test.isPending || (needsToken && !integration.has_token && !token)} onClick={() => test.mutate({ service: integration.service, input: buildInput(integration, values, token) }, { onSuccess: setResult, onError: toastError })}>Test connection</Button>
        {!readOnly && <Button disabled={!integration.is_enabled} onClick={() => sync.mutate({ service: integration.service }, { onSuccess: () => toast('Sync started'), onError: toastError })}>Sync now</Button>}
      </div>
      {result && <div role="status" className={result.ok ? 'small' : 'err-text'}>{result.message}</div>}

      {excluded.length > 0 && (
        <div className="col">
          <h3>Excluded items</h3>
          {excluded.map((x) => {
            const label = x.title ?? x.id;
            return (
              <div key={x.id} className="row between">
                <span className="clamp">{label}</span>
                {!readOnly && <Button size="sm" onClick={() => restore.mutate({ service: integration.service, id: x.id }, { onError: toastError })} aria-label={`Restore ${label}`}>Restore</Button>}
              </div>
            );
          })}
        </div>
      )}

      <div className="col">
        <h3>Activity</h3>
        {logs.length === 0 && <span className="small muted">No recent activity</span>}
        {logs.map((l) => (
          <div key={l.id} className="small">
            <b>{l.status}</b> · {l.error_message ?? `${l.entries_created} new, ${l.entries_updated} updated`} <span className="muted">{relativeTime(l.completed_at ?? l.started_at)}</span>
          </div>
        ))}
      </div>

      {!readOnly && state !== 'not_connected' && (
        <div>
          <Button variant="danger" onClick={() => setResetting(true)}>Reset integration</Button>
          <Dialog open={resetting} onOpenChange={setResetting} title={`Reset ${integration.display_name}`} width={440}>
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
  return (
    <Drawer open={service != null} onOpenChange={(o) => !o && onClose()} title={integration?.display_name ?? service ?? ''} subtitle="Integration settings">
      {integration && <Form key={integration.service} integration={integration} onClose={onClose} />}
    </Drawer>
  );
}
