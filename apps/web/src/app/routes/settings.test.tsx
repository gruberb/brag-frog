import { describe, expect, it } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Route, Routes } from 'react-router';
import { db } from '@/testing/mocks/db';
import { renderWithProviders } from '@/testing/test-utils';
import { Component as Settings } from './settings';

const ui = (
  <Routes>
    <Route path="/settings/:tab?" element={<Settings />} />
  </Routes>
);

describe('Settings', () => {
  it('renders the tab from the URL and switches tabs', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/cycles' });
    expect(await screen.findByText('Performance cycles')).toBeInTheDocument();
    await user.click(screen.getByRole('tab', { name: 'People' }));
    expect(await screen.findByText('sam@example.com', { exact: false })).toBeInTheDocument();
  });

  it('makes a past cycle active', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/cycles' });
    await user.click(await screen.findByRole('button', { name: 'Make active' }));
    await waitFor(() => expect(db.cycles.find((c) => c.id === 1)?.is_active).toBe(true));
  });

  it('lists integrations, tests a connection and starts a sync', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/integrations' });
    expect(await screen.findByText(/last sync failed: token expired/)).toBeInTheDocument();
    expect(screen.queryByText('claude')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Configure GitHub' }));
    const drawer = await screen.findByRole('dialog', { name: 'GitHub' });
    await user.click(within(drawer).getByRole('button', { name: 'Test connection' }));
    expect(await within(drawer).findByText('Connection works')).toBeInTheDocument();
    await user.click(within(drawer).getByRole('button', { name: 'Close' }));
    await user.click(screen.getByRole('button', { name: 'Sync all' }));
    expect(await screen.findByRole('button', { name: 'Syncing…' })).toBeDisabled();
  });

  it('restores an excluded calendar item', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/integrations' });
    await user.click(await screen.findByRole('button', { name: 'Configure Google Calendar' }));
    await user.click(await screen.findByRole('button', { name: 'Restore Lunch' }));
    await waitFor(() => expect(db.integrations.find((i) => i.service === 'google_calendar')?.excluded_events).toHaveLength(1));
  });
});
