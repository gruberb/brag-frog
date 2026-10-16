import { describe, expect, it } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/testing/mocks/server';
import { db } from '@/testing/mocks/db';
import { renderWithProviders } from '@/testing/test-utils';
import { Component as Today } from './today';

describe('Today', () => {
  it('shows the week, priorities and open work', async () => {
    renderWithProviders(<Today />, { route: '/today' });
    expect(await screen.findByText('Platform sync')).toBeInTheDocument();
    expect(screen.getByText(/meetings · 11h focus time/)).toBeInTheDocument();
    expect(screen.getByText('Reduce CI time by 30%', { selector: 'b' })).toBeInTheDocument();
    expect(screen.getByText('At risk')).toBeInTheDocument();
    expect(screen.getAllByText('prep ready').length).toBeGreaterThan(0);
    expect(screen.getByRole('heading', { name: 'Open work' })).toBeInTheDocument();
  });

  it('filters open work by tab', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Today />, { route: '/today' });
    await screen.findByText('Platform sync');
    const all = document.querySelectorAll('.list > .row').length;
    await user.click(screen.getByRole('button', { name: 'Reviews' }));
    expect(document.querySelectorAll('.list > .row').length).toBeLessThan(all);
  });

  it('logs an entry from the quick composer and shows it in the logbook data', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Today />, { route: '/today' });
    const input = await screen.findByLabelText('Log something you did');
    await user.type(input, 'Wrote the rollout plan{Enter}');
    await waitFor(() => expect(db.entries[0].title).toBe('Wrote the rollout plan'));
    expect(input).toHaveValue('');
  });

  it('saves meeting prep from the drawer', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Today />, { route: '/today' });
    await user.click(await screen.findByRole('button', { name: /Platform sync/ }));
    const drawer = await screen.findByRole('dialog');
    await user.type(await within(drawer).findByLabelText('Goal of this meeting'), 'Agree on nodes');
    await user.click(within(drawer).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(db.preps[7000]?.meeting_goal).toBe('Agree on nodes'));
  });

  it('inserts an AI draft into the notes', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Today />, { route: '/today' });
    await user.click(await screen.findByRole('button', { name: /Platform sync/ }));
    const drawer = await screen.findByRole('dialog');
    await user.click(await within(drawer).findByRole('button', { name: 'Draft with AI' }));
    await waitFor(() => expect((within(drawer).getByLabelText('Notes') as HTMLTextAreaElement).value).toContain('Confirm runner capacity'));
  });

  it('hides the composer for past cycles', async () => {
    renderWithProviders(<Today />, { route: '/today', readOnly: true });
    await screen.findByText('Platform sync');
    expect(screen.queryByLabelText('Log something you did')).not.toBeInTheDocument();
  });

  it('shows an error with retry when the API fails', async () => {
    server.use(http.get('/api/v1/today', () => HttpResponse.json({ error: { code: 'internal', message: 'Nope' } }, { status: 500 })));
    renderWithProviders(<Today />, { route: '/today' });
    expect(await screen.findByText('Nope')).toBeInTheDocument();
  });
});
