import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { toastError } from '@/stores/toast';
import type { ImportResult } from '@/types/api/models';
import { useImportLattice } from '../api/priorities';

export function ImportDialog({ open, onOpenChange, cycleId }: { open: boolean; onOpenChange: (o: boolean) => void; cycleId: number | null }) {
  const imp = useImportLattice();
  const [file, setFile] = useState<File | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  return (
    <Dialog open={open} onOpenChange={(o) => { onOpenChange(o); if (!o) { setResult(null); setFile(null); } }} title="Import from Lattice" width={480}>
      <div className="card-b col" style={{ gap: 12 }}>
        <p className="small muted" style={{ margin: 0 }}>Upload the CSV exported from Lattice. Existing priorities with the same title are updated.</p>
        <input type="file" accept=".csv,text/csv" aria-label="Lattice CSV" onChange={(e) => { setFile(e.target.files?.[0] ?? null); setResult(null); }} />
        {result && (
          <div role="status" className="col" style={{ gap: 4 }}>
            <b>{result.created} created · {result.updated} updated · {result.skipped} skipped</b>
            {result.errors.length > 0 && <ul style={{ margin: 0, paddingLeft: 18 }}>{result.errors.map((e) => <li key={e} className="err-text">{e}</li>)}</ul>}
          </div>
        )}
        <div className="row" style={{ justifyContent: 'flex-end' }}>
          <Button onClick={() => onOpenChange(false)}>{result ? 'Close' : 'Cancel'}</Button>
          <Button variant="primary" disabled={!file || cycleId == null || imp.isPending} onClick={() => file && cycleId != null && imp.mutate({ file, cycleId }, { onSuccess: setResult, onError: toastError })}>
            {imp.isPending ? 'Importing…' : 'Import'}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
