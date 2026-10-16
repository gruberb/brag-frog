import { describe, expect, it } from 'vitest';
import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/testing/mocks/server';

import { renderWithProviders } from '@/testing/test-utils';
import { Component as Priorities } from './priorities';

describe('Priorities', () => {
  it('groups priorities by department goal and filters Active/Done', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Priorities />, { route: '/priorities' });
    expect(await screen.findByRole('region', { name: 'Developer velocity' })).toBeInTheDocument();
    expect(screen.getByText('Reduce CI time by 30%')).toBeInTheDocument();
    expect(screen.queryByText('Quota alerts rollout')).not.toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'No goal' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /^Done/ }));
    expect(await screen.findByText('Quota alerts rollout')).toBeInTheDocument();
    expect(screen.queryByText('Reduce CI time by 30%')).not.toBeInTheDocument();
  });

  it('shows the import result list', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Priorities />, { route: '/priorities' });
    await user.click(await screen.findByRole('button', { name: 'Import from Lattice' }));
    const dlg = await screen.findByRole('dialog');
    await user.upload(within(dlg).getByLabelText('Lattice CSV'), new File(['title\nA'], 'p.csv', { type: 'text/csv' }));
    await user.click(within(dlg).getByRole('button', { name: 'Import' }));
    expect(await within(dlg).findByText(/2 created · 1 updated · 1 skipped/)).toBeInTheDocument();
    expect(within(dlg).getByText('Row 5: missing title')).toBeInTheDocument();
  });

  it('is read-only for past cycles', async () => {
    renderWithProviders(<Priorities />, { route: '/priorities', readOnly: true });
    await screen.findByText('Reduce CI time by 30%');
    expect(screen.queryByRole('button', { name: '+ Priority' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Complete goal' })).not.toBeInTheDocument();
  });

  it('shows an error state', async () => {
    server.use(http.get('/api/v1/priorities', () => HttpResponse.json({ error: { code: 'internal', message: 'Down' } }, { status: 500 })));
    renderWithProviders(<Priorities />, { route: '/priorities' });
    expect(await screen.findByText('Down')).toBeInTheDocument();
  });
});
