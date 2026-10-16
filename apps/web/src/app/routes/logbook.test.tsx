import { describe, expect, it } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/testing/mocks/server';
import { renderWithProviders } from '@/testing/test-utils';
import { db } from '@/testing/mocks/db';
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

  it('edits an entry through the drawer', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Logbook />, { route: '/logbook' });
    await user.click(await screen.findByText('Merged runner autoscaling'));
    const title = await screen.findByLabelText('Title');
    await user.clear(title);
    await user.type(title, 'Merged autoscaling v2');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByText('Entry saved')).toBeInTheDocument();
    expect(db.entries.find((e) => e.title === 'Merged autoscaling v2')).toBeTruthy();
    await waitFor(() => expect(screen.getByText('Merged autoscaling v2')).toBeInTheDocument());
  });

  it('shows a validation error instead of saving an empty title', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Logbook />, { route: '/logbook' });
    await user.click(await screen.findByText('Merged runner autoscaling'));
    await user.clear(await screen.findByLabelText('Title'));
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByText('Title is required')).toBeInTheDocument();
  });

  it('asks for confirmation before deleting', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Logbook />, { route: '/logbook' });
    await user.click(await screen.findByText('Mentored new hire on storage internals'));
    await user.click(await screen.findByRole('button', { name: 'Delete' }));
    const dialog = await screen.findByRole('dialog', { name: 'Delete entry' });
    expect(db.entries.some((e) => e.title === 'Mentored new hire on storage internals')).toBe(true);
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(db.entries.some((e) => e.title === 'Mentored new hire on storage internals')).toBe(false));
  });

  it('hides mutations in a past cycle', async () => {
    renderWithProviders(<Logbook />, { route: '/logbook', readOnly: true });
    await screen.findByText(/entries/);
    expect(screen.queryByRole('button', { name: 'Log entry' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Bulk Edit' })).not.toBeInTheDocument();
  });

  it('shows an error state with retry', async () => {
    server.use(http.get('/api/v1/entries', () => HttpResponse.json({ error: { code: 'internal', message: 'Boom' } }, { status: 500 })));
    renderWithProviders(<Logbook />, { route: '/logbook' });
    expect(await screen.findByText('Boom')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });
});
