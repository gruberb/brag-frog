import { describe, expect, it } from 'vitest';
import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Route, Routes } from 'react-router';
import { db } from '@/testing/mocks/db';
import { renderWithProviders } from '@/testing/test-utils';
import { Component as Review } from './review';

const ui = (
  <Routes>
    <Route path="/review/:cycleId?/:section?" element={<Review />} />
  </Routes>
);

describe('Review', () => {
  it('renders section pills with empty status, the hint and the converted old text', async () => {
    renderWithProviders(ui, { route: '/review/2' });
    expect(await screen.findByRole('tab', { name: 'Impact examples' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: /Growth\s*empty/ })).toBeInTheDocument();
    expect(screen.getByText(/Describe up to 4 examples/)).toBeInTheDocument();
    // Old markdown is converted into blocks on open and offered as "view original".
    expect(await screen.findByText(/Cut CI time by 45%/, {}, { timeout: 8000 })).toBeInTheDocument();
    expect(screen.getByText(/Converted from your earlier text/)).toBeInTheDocument();
  }, 15000);

  it('switches sections through the URL', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/review/2' });
    await user.click(await screen.findByRole('tab', { name: /Growth/ }));
    expect(await screen.findByRole('tab', { name: /Growth/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText(/How did you grow\?/)).toBeInTheDocument();
  });

  it('expands guidance inline and opens the career level drawer', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/review/2' });
    await user.click(await screen.findByRole('button', { name: 'Guidance' }));
    expect(screen.getByText('The outcome or progress made')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Career level' }));
    expect(await screen.findByRole('dialog', { name: 'Senior Engineer (IC5)' })).toBeInTheDocument();
  });

  it('opens the evidence drawer closed-by-default and lists entries with Insert', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/review/2' });
    expect(screen.queryByRole('dialog', { name: 'Add evidence' })).not.toBeInTheDocument();
    await user.click(await screen.findByRole('button', { name: /Add evidence from your logbook/ }, { timeout: 8000 }));
    const drawer = await screen.findByRole('dialog', { name: 'Add evidence' });
    await within(drawer).findByText('Merged runner autoscaling');
    expect(within(drawer).getAllByRole('button', { name: /^Insert/ }).length).toBeGreaterThan(0);
  }, 15000);

  it('drafts with AI and shows the draft in the editor', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/review/2/growth' });
    await screen.findByText(/How did you grow\?/);
    await user.click(await screen.findByRole('button', { name: 'Draft with AI' }));
    await user.click(await screen.findByRole('button', { name: 'Generate draft' }));
    expect(await screen.findByText(/Drafted example/, {}, { timeout: 8000 })).toBeInTheDocument();
  }, 20000);

  it('is read-only for a past cycle and disables drafting', async () => {
    renderWithProviders(ui, { route: '/review/1', readOnly: true });
    await screen.findByRole('tab', { name: /Impact examples/ });
    expect(screen.queryByRole('button', { name: 'Draft with AI' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Add evidence/ })).not.toBeInTheDocument();
  });

  it('shows the archive', async () => {
    renderWithProviders(ui, { route: '/review/2/archive' });
    expect(await screen.findByText('Q2 check-in')).toBeInTheDocument();
    expect(screen.getByText('Autoscaling')).toBeInTheDocument();
    expect(db.archive.checkins).toHaveLength(1);
  });
});
