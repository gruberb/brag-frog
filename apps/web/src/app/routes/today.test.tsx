import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/testing/mocks/server';

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
