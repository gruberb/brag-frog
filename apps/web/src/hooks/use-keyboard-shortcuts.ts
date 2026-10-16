import { useEffect, useRef } from 'react';

export type ShortcutMap = {
  /** Key after "g", for example { t: () => navigate('/today') }. */
  chords?: Record<string, () => void>;
  onPalette?: () => void;
  onComposer?: () => void;
};

const CHORD_WINDOW_MS = 1000;

function isTyping(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  if (!el) return false;
  return el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable || !!el.closest?.('.bn-editor');
}

/** ⌘K palette, ⌘J composer, and "g" then a letter to navigate. Ignored while typing in a field or the editor. */
export function useKeyboardShortcuts(map: ShortcutMap) {
  const ref = useRef(map);
  useEffect(() => {
    ref.current = map;
  });
  useEffect(() => {
    let gAt = 0;
    const onKey = (e: KeyboardEvent) => {
      const m = ref.current;
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        m.onPalette?.();
        return;
      }
      if (mod && e.key.toLowerCase() === 'j') {
        e.preventDefault();
        m.onComposer?.();
        return;
      }
      if (mod || e.altKey || isTyping(e.target)) return;
      const key = e.key.toLowerCase();
      if (gAt && Date.now() - gAt < CHORD_WINDOW_MS) {
        gAt = 0;
        const fn = m.chords?.[key];
        if (fn) {
          e.preventDefault();
          fn();
        }
        return;
      }
      if (key === 'g') gAt = Date.now();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
}
