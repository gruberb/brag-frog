import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { screen, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderApp } from '@/testing/test-utils';
import { server } from '@/testing/mocks/server';

describe('app shell', () => {
  it('renders the sidebar with the five nav items and a settings gear', async () => {
    renderApp('/today');
    const nav = await screen.findByRole('navigation', { name: 'Main' });
    expect(within(nav).getAllByRole('link').map((a) => a.textContent)).toEqual(['Today', 'Logbook', 'Priorities', 'Review', 'Trends']);
    expect(screen.getByRole('link', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.getByText('bgruber@example.com')).toBeInTheDocument();
  });

  it('redirects to /signin on 401', async () => {
    server.use(http.get('/api/v1/me', () => HttpResponse.json({ error: { code: 'unauthorized', message: 'Sign in' } }, { status: 401 })));
    const { router } = renderApp('/today');
    await screen.findByRole('link', { name: 'Sign in with Google' });
    expect(router.state.location.pathname).toBe('/signin');
  });

  it('shows the not-found page for unknown paths', async () => {
    renderApp('/nope');
    expect(await screen.findByText('Page not found')).toBeInTheDocument();
  });
});

// Brand fonts are fixed: only the token stack may appear in any stylesheet or component.
describe('font guard', () => {
  const walk = (dir: string): string[] => readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? walk(p) : [p];
  });
  const src = join(__dirname, '..');
  // PDF/DOCX documents embed IBM Plex Sans and Courier: that is document output, not the app UI.
  const files = walk(src).filter((f) => /\.(css|tsx?)$/.test(f) && !f.includes('.test.') && !f.includes('/types/api/') && !f.includes('/lib/export/'));

  it('declares Mozilla Text first in the body font stack', () => {
    const css = readFileSync(join(src, 'app/app.css'), 'utf8');
    expect(css).toMatch(/--font-stack:\s*'Mozilla Text'/);
    expect(css).toMatch(/--font-stack-heading:\s*'Mozilla Headline'/);
    expect(readFileSync(join(src, 'app/components.css'), 'utf8')).toMatch(/body \{ font-family: var\(--font-stack\)/);
  });

  it('uses no other font-family than the tokens', () => {
    const offenders = files.flatMap((f) =>
      readFileSync(f, 'utf8').split('\n').filter((l) => /font-?[fF]amily/.test(l) && !/var\(--font-stack|inherit|--bn-font-family|font-family: 'Mozilla|--font-/.test(l)).map((l) => `${f}: ${l.trim()}`),
    );
    expect(offenders).toEqual([]);
  });

  it('keeps form controls on font: inherit', () => {
    expect(readFileSync(join(src, 'app/components.css'), 'utf8')).toMatch(/input, select, textarea, button \{ font: inherit/);
  });
});
