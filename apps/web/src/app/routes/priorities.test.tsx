import { describe, expect, it } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/testing/mocks/server';
import { db } from '@/testing/mocks/db';
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

  it('posts an update with a blocker and shows it in the timeline', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Priorities />, { route: '/priorities' });
    await user.click(await screen.findByText('Ship workspace API v2'));
    const drawer = await screen.findByRole('dialog');
    await user.type(within(drawer).getByLabelText('Update'), 'Waiting on the schema review');
    await user.click(within(drawer).getByLabelText('Blocker'));
    await user.click(within(drawer).getByRole('button', { name: 'Post update' }));
    expect(await within(drawer).findByText('Waiting on the schema review')).toBeInTheDocument();
    expect(db.updates.find((u) => u.comment === 'Waiting on the schema review')?.is_blocker).toBe(true);
  });

  it('requires a comment before posting', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Priorities />, { route: '/priorities' });
    await user.click(await screen.findByText('Ship workspace API v2'));
    await user.click(await screen.findByRole('button', { name: 'Post update' }));
    expect(await screen.findByText('Say what changed')).toBeInTheDocument();
  });

  it('creates a priority and rejects an empty title', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Priorities />, { route: '/priorities' });
    await user.click(await screen.findByRole('button', { name: '+ Priority' }));
    const dlg = await screen.findByRole('dialog', { name: 'New priority' });
    await user.click(within(dlg).getByRole('button', { name: 'Create priority' }));
    expect(await within(dlg).findByText('Title is required')).toBeInTheDocument();
    await user.type(within(dlg).getByLabelText('Title'), 'Improve onboarding docs');
    await user.click(within(dlg).getByRole('button', { name: 'Create priority' }));
    await waitFor(() => expect(db.priorities.some((p) => p.title === 'Improve onboarding docs')).toBe(true));
    expect(await screen.findByText('Improve onboarding docs')).toBeInTheDocument();
  });

  it('completes a goal and its priorities', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Priorities />, { route: '/priorities' });
    const region = await screen.findByRole('region', { name: 'Developer velocity' });
    await user.click(within(region).getByRole('button', { name: 'Complete goal' }));
    await waitFor(() => expect(db.priorities.find((p) => p.id === 11)?.status).toBe('completed'));
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
