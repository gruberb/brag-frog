import '@testing-library/jest-dom/vitest';
import { afterAll, afterEach, beforeAll } from 'vitest';
import { cleanup } from '@testing-library/react';
import { server } from './mocks/server';
import { resetDb } from './mocks/db';

// jsdom lacks these; Radix and layout code touch them. Skipped for the few files that run in the node environment.
if (typeof window !== 'undefined') {
  class RO { observe() {} unobserve() {} disconnect() {} }
  globalThis.ResizeObserver ??= RO as unknown as typeof ResizeObserver;
  Element.prototype.scrollIntoView ??= () => {};
  // ProseMirror (BlockNote) probes hit-testing, which jsdom does not implement.
  document.elementsFromPoint ??= () => [];
  document.elementFromPoint ??= () => null;
  Element.prototype.hasPointerCapture ??= () => false;
  Element.prototype.releasePointerCapture ??= () => {};
  window.matchMedia ??= ((q: string) => ({ matches: false, media: q, addEventListener() {}, removeEventListener() {}, addListener() {}, removeListener() {}, dispatchEvent: () => false, onchange: null })) as typeof window.matchMedia;
}

beforeAll(() => server.listen({ onUnhandledRequest: 'error' }));
afterEach(() => {
  cleanup();
  server.resetHandlers();
  resetDb();
});
afterAll(() => server.close());
