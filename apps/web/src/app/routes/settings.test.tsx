import { describe, expect, it } from 'vitest';
import { fireEvent, screen, waitFor, within } from '@testing-library/react';
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

  it('validates and saves the profile', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings' });
    const name = await screen.findByLabelText('Display name');
    await user.clear(name);
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByText('Display name is required')).toBeInTheDocument();
    await user.type(name, 'Bas');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(db.profile.display_name).toBe('Bas'));
  });

  it('creates a cycle and rejects an end date before the start', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/cycles' });
    await user.click(await screen.findByRole('button', { name: /New cycle/ }));
    const dlg = await screen.findByRole('dialog', { name: 'New cycle' });
    await user.type(within(dlg).getByLabelText('Name'), 'H1 2027');
    fireEvent.change(within(dlg).getByLabelText('End'), { target: { value: '2025-12-31' } });
    await user.click(within(dlg).getByRole('button', { name: 'Create cycle' }));
    expect(await within(dlg).findByText('End must be on or after the start')).toBeInTheDocument();
    fireEvent.change(within(dlg).getByLabelText('End'), { target: { value: '2027-06-30' } });
    await user.click(within(dlg).getByRole('button', { name: 'Create cycle' }));
    await waitFor(() => expect(db.cycles.some((c) => c.name === 'H1 2027')).toBe(true));
  });

  it('makes a past cycle active', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/cycles' });
    await user.click(await screen.findByRole('button', { name: 'Make active' }));
    await waitFor(() => expect(db.cycles.find((c) => c.id === 1)?.is_active).toBe(true));
  });

  it('only deletes a cycle after the exact name is typed', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/cycles' });
    await user.click(await screen.findByRole('button', { name: 'Delete' }));
    const dlg = await screen.findByRole('dialog', { name: 'Delete H1 2026' });
    const confirm = within(dlg).getByRole('button', { name: 'Delete' });
    expect(confirm).toBeDisabled();
    await user.type(within(dlg).getByLabelText(/Type/), 'H1 2026');
    expect(confirm).toBeEnabled();
    await user.click(confirm);
    await waitFor(() => expect(db.cycles.some((c) => c.id === 1)).toBe(false));
  });

  it('adds a person and rejects an invalid email', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/people' });
    await user.click(await screen.findByRole('button', { name: /Add$/ }));
    await user.type(screen.getByLabelText('Email'), 'nope');
    await user.type(screen.getByLabelText('Name'), 'Jo');
    await user.click(screen.getByRole('button', { name: 'Add person' }));
    expect(await screen.findByText('Enter a valid email')).toBeInTheDocument();
    await user.clear(screen.getByLabelText('Email'));
    await user.type(screen.getByLabelText('Email'), 'jo@example.com');
    await user.click(screen.getByRole('button', { name: 'Add person' }));
    await waitFor(() => expect(db.people.some((p) => p.email === 'jo@example.com')).toBe(true));
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

  it('saves the AI key', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/settings/ai' });
    await user.type(await screen.findByLabelText('Anthropic API key'), 'sk-test-value');
    await user.click(screen.getByRole('button', { name: 'Save key' }));
    expect(await screen.findByText('Key saved', { selector: '.toast' })).toBeInTheDocument();
  });
});
