import { describe, expect, it } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/testing/mocks/server';
import { renderWithProviders } from '@/testing/test-utils';

import { Component as Logbook } from './logbook';

const seen: URL[] = [];
const spy = () => {
  seen.length = 0;
  server.use(http.get('/api/v1/entries', ({ request }) => { seen.push(new URL(request.url)); return undefined; }));
};

describe('Logbook', () => {
  it('lists entries with day groups and a total', async () => {
    renderWithProviders(<Logbook />, { route: '/logbook' });
    expect(await screen.findByText('Merged runner autoscaling')).toBeInTheDocument();
    expect(screen.getByText(/90 of 90 entries/)).toBeInTheDocument();
    expect(screen.getAllByText('Today')[0]).toBeInTheDocument();
  });

  it('sends URL filters to the API and debounces search', async () => {
    spy();
    const user = userEvent.setup();
    renderWithProviders(<Logbook />, { route: '/logbook?source=github&priority_id=11' });
    await screen.findByText('Merged runner autoscaling');
    expect(seen.at(-1)!.searchParams.get('source')).toBe('github');
    expect(seen.at(-1)!.searchParams.get('priority_id')).toBe('11');
    expect(seen.at(-1)!.searchParams.get('cycle_id')).toBe('2');

    await user.type(screen.getByLabelText('Search entries'), 'quota');
    await waitFor(() => expect(seen.at(-1)!.searchParams.get('q')).toBe('quota'));
    // One request for the final text, not one per keystroke.
    expect(seen.filter((u) => u.searchParams.get('q')).length).toBe(1);
  });

  it('loads further pages with a cursor', async () => {
    spy();
    const user = userEvent.setup();
    renderWithProviders(<Logbook />, { route: '/logbook' });
    await screen.findByText('Merged runner autoscaling');
    const rows = () => document.querySelectorAll('.entry').length;
    expect(rows()).toBe(50);
    await user.click(await screen.findByRole('button', { name: 'Load more' }));
    await waitFor(() => expect(seen.some((u) => u.searchParams.get('cursor') === '50')).toBe(true));
    await waitFor(() => expect(rows()).toBe(90));
    expect(screen.queryByRole('button', { name: 'Load more' })).not.toBeInTheDocument();
  });

  it('hides mutations in a past cycle', async () => {
    renderWithProviders(<Logbook />, { route: '/logbook', readOnly: true });
    await screen.findByText(/entries/);
    expect(screen.queryByRole('button', { name: '+ Log entry' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Bulk edit' })).not.toBeInTheDocument();
  });

  it('shows an error state with retry', async () => {
    server.use(http.get('/api/v1/entries', () => HttpResponse.json({ error: { code: 'internal', message: 'Boom' } }, { status: 500 })));
    renderWithProviders(<Logbook />, { route: '/logbook' });
    expect(await screen.findByText('Boom')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });
});
