import { useCallback, useEffect, useLayoutEffect, useRef } from 'react';

type Options = {
  delayMs?: number;
  onError?: (err: unknown) => void;
};

/**
 * Debounces saves (default 500 ms) and flushes the pending one on unmount, page hide and tab hide,
 * so the last keystrokes survive route changes and closing the tab.
 * `save` receives the latest value passed to `schedule`.
 */
export function useAutoSave<T>(save: (value: T) => Promise<unknown>, { delayMs = 500, onError }: Options = {}) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latest = useRef<{ value: T } | null>(null);
  const inFlight = useRef(0);
  const saveRef = useRef(save);
  const errRef = useRef(onError);
  useEffect(() => {
    saveRef.current = save;
    errRef.current = onError;
  });

  const run = useCallback(() => {
    timer.current = null;
    const pending = latest.current;
    if (!pending) return;
    latest.current = null;
    inFlight.current++;
    saveRef.current(pending.value)
      .catch((e) => errRef.current?.(e))
      .finally(() => {
        inFlight.current--;
      });
  }, []);

  const flush = useCallback(() => {
    if (!timer.current) return;
    clearTimeout(timer.current);
    run();
  }, [run]);

  const schedule = useCallback(
    (value: T) => {
      latest.current = { value };
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(run, delayMs);
    },
    [run, delayMs],
  );

  /** True while typing waits to be saved or a save is in flight. */
  const isPending = useCallback(() => timer.current !== null || inFlight.current > 0, []);

  useEffect(() => {
    const onHide = () => document.visibilityState === 'hidden' && flush();
    window.addEventListener('pagehide', flush);
    document.addEventListener('visibilitychange', onHide);
    return () => {
      window.removeEventListener('pagehide', flush);
      document.removeEventListener('visibilitychange', onHide);
    };
  }, [flush]);

  // Layout cleanup runs before child components tear down, so the value is still readable.
  useLayoutEffect(() => flush, [flush]);

  return { schedule, flush, isPending };
}
