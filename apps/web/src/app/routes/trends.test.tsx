import { describe, expect, it } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { Route, Routes, useLocation } from 'react-router';
import { server } from '@/testing/mocks/server';
import { renderWithProviders } from '@/testing/test-utils';
import { Component as Trends } from './trends';

const Where = () => {
  const l = useLocation();
  return <div data-testid="where">{l.pathname + l.search}</div>;
};
const ui = (
  <Routes>
    <Route path="/trends" element={<Trends />} />
    <Route path="/logbook" element={<Where />} />
  </Routes>
);

describe('Trends', () => {
  it('renders KPIs, the weekly chart, priority share and ranked lists', async () => {
    renderWithProviders(ui, { route: '/trends' });
    expect(await screen.findByText('people worked with')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Activity by week' })).toBeInTheDocument();
    expect(screen.getByText('Priority share')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Sam: \d+\. Open in logbook/ })).toBeInTheDocument();
  });

  it('switches range through the URL and requests it', async () => {
    const seen: string[] = [];
    server.use(http.get('/api/v1/trends', ({ request }) => { seen.push(new URL(request.url).searchParams.get('range')!); return undefined; }));
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/trends' });
    await screen.findByText('people worked with');
    await user.click(screen.getByRole('button', { name: '4 weeks' }));
    await waitFor(() => expect(seen).toContain('4w'));
    expect(screen.getByRole('button', { name: '4 weeks' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('clicking a person opens the logbook filtered by that person', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/trends' });
    await user.click(await screen.findByRole('link', { name: /Sam: \d+/ }));
    expect(await screen.findByTestId('where')).toHaveTextContent('/logbook?person=Sam');
  });

  it('clicking a week bar opens that week in the logbook', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/trends' });
    await screen.findByText('people worked with');
    const bars = within(screen.getByRole('group', { name: 'Activity by week' })).getAllByRole('link');
    await user.click(bars.at(-1)!);
    expect(await screen.findByTestId('where')).toHaveTextContent(/\/logbook\?from=\d{4}-\d{2}-\d{2}&to=\d{4}-\d{2}-\d{2}/);
  });

  it('offers the data behind each chart in a table', async () => {
    const user = userEvent.setup();
    renderWithProviders(ui, { route: '/trends' });
    await screen.findByText('people worked with');
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    await user.click(screen.getAllByRole('button', { name: 'Show data' })[0]);
    expect(screen.getByRole('columnheader', { name: 'Week' })).toBeInTheDocument();
  });

  it('shows an error with retry', async () => {
    server.use(http.get('/api/v1/trends', () => HttpResponse.json({ error: { code: 'internal', message: 'Slow day' } }, { status: 500 })));
    renderWithProviders(ui, { route: '/trends' });
    expect(await screen.findByText('Slow day')).toBeInTheDocument();
  });
});
