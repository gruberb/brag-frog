import { useSyncExternalStore } from 'react';

export type Toast = { id: number; message: string; kind: 'info' | 'error' };

let toasts: Toast[] = [];
let nextId = 1;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());

export function toast(message: string, kind: Toast['kind'] = 'info', ttlMs = 4000) {
  const id = nextId++;
  toasts = [...toasts, { id, message, kind }];
  emit();
  setTimeout(() => dismissToast(id), ttlMs);
}
export const toastError = (err: unknown) => toast(err instanceof Error ? err.message : 'Something went wrong', 'error', 6000);

export function dismissToast(id: number) {
  toasts = toasts.filter((t) => t.id !== id);
  emit();
}

export function useToasts() {
  return useSyncExternalStore(
    (cb) => (listeners.add(cb), () => listeners.delete(cb)),
    () => toasts,
  );
}
