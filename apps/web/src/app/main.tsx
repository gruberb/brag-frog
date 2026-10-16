import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './app.css';
import { App } from './app';

// `npm run dev:mock` serves the whole UI from in-memory MSW data, no backend needed.
async function enableMocking() {
  if (import.meta.env.VITE_MSW !== 'true') return;
  const { worker } = await import('@/testing/mocks/browser');
  await worker.start({ onUnhandledRequest: 'bypass', quiet: true });
}

void enableMocking().then(() =>
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  ),
);
