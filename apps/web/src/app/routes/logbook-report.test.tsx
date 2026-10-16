import { beforeEach, describe, expect, it, vi } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/testing/mocks/server';
import { db } from '@/testing/mocks/db';
import { renderWithProviders } from '@/testing/test-utils';
import { Component as Logbook } from './logbook';

const downloadReport = vi.fn().mockResolvedValue(undefined);
vi.mock('@/lib/export', () => ({ downloadReport: (...a: unknown[]) => downloadReport(...a) }));

beforeEach(() => downloadReport.mockClear());

async function openDialog(route = '/logbook', opts: Parameters<typeof renderWithProviders>[1] = {}) {
  const user = userEvent.setup();
  renderWithProviders(<Logbook />, { route, ...opts });
  await screen.findByText(/entries/);
  await user.click(screen.getByRole('button', { name: 'Report' }));
  const dlg = await screen.findByRole('dialog', { name: 'Create report' });
  return { user, dlg };
}

describe('Logbook report dialog', () => {
  it('shows the active filters as chips and the matching count', async () => {
    const { dlg } = await openDialog('/logbook?priority_id=11&from=2026-10-01');
    expect(await within(dlg).findByText('Reduce CI time by 30%')).toBeInTheDocument();
    expect(within(dlg).getByText(/Oct 1/)).toBeInTheDocument();
    await waitFor(() => expect(within(dlg).getByText(/\d+ entries/)).toBeInTheDocument());
    expect(within(dlg).getByLabelText('Title')).toHaveAttribute('placeholder', expect.stringContaining('Reduce CI time by 30%'));
  });

  it('generates, saves and downloads in the chosen format with the URL filters', async () => {
    const { user, dlg } = await openDialog('/logbook?priority_id=11');
    await user.click(within(dlg).getByRole('button', { name: 'DOCX' }));
    await user.click(within(dlg).getByRole('button', { name: 'Generate and download' }));
    await waitFor(() => expect(downloadReport).toHaveBeenCalledTimes(1));
    expect(downloadReport.mock.calls[0][0]).toBe('docx');
    expect(db.reports).toHaveLength(1);
    expect(db.reports[0].filters).toMatchObject({ priority_id: 11 });
    expect(db.reports[0].narrative.content).not.toBe(''); // AI box was checked
  });

  it('hides the AI option when no key is configured', async () => {
    const { dlg } = await openDialog('/logbook', { me: { has_ai: false } });
    expect(within(dlg).queryByLabelText('Include an AI summary')).not.toBeInTheDocument();
  });

  it('disables generating when no entry matches the filters', async () => {
    const { dlg } = await openDialog('/logbook?q=zzz-nothing');
    await waitFor(() => expect(within(dlg).getByText(/0 entries/)).toBeInTheDocument());
    expect(within(dlg).getByRole('button', { name: 'Generate and download' })).toBeDisabled();
  });

  it('surfaces the server empty_selection rule inline', async () => {
    server.use(http.post('/api/v1/reports', () => HttpResponse.json({ error: { code: 'empty_selection', message: 'none' } }, { status: 422 })));
    const { user, dlg } = await openDialog('/logbook');
    await user.click(within(dlg).getByRole('button', { name: 'Generate and download' }));
    expect(await within(dlg).findByRole('alert')).toHaveTextContent('No entries match the current filters');
    expect(downloadReport).not.toHaveBeenCalled();
  });

  it('keeps a report without an AI summary and says so when AI fails', async () => {
    server.use(http.post('/api/v1/reports', () => HttpResponse.json({
      id: 99, title: 'T', filters: {}, entry_count: 1, generated_at: '2026-10-16T00:00:00Z', ai_error: 'rate limited', narrative: { content: '', content_format: 'markdown' }, key_entries: [],
      stats: { entries: 1, people: 0, repos: 0, teams: 0, per_week: [], per_project: [], people_top: [], repos_top: [], teams_top: [] },
    }, { status: 201 })));
    const { user, dlg } = await openDialog('/logbook');
    await user.click(within(dlg).getByRole('button', { name: 'Generate and download' }));
    expect(await within(dlg).findByText(/without an AI summary: rate limited/)).toBeInTheDocument();
    expect(downloadReport).toHaveBeenCalled();
  });

  it('lists past reports, re-downloads one and deletes one after confirmation', async () => {
    const { user, dlg } = await openDialog('/logbook');
    await user.click(within(dlg).getByRole('button', { name: 'Generate and download' }));
    await waitFor(() => expect(db.reports).toHaveLength(1));
    downloadReport.mockClear();
    // Generating closes the dialog; reopen it for the Past tab.
    await user.click(await screen.findByRole('button', { name: 'Report' }));
    const dlg2 = await screen.findByRole('dialog', { name: 'Create report' });
    await user.click(within(dlg2).getByRole('tab', { name: /Past reports/ }));
    await user.click(await within(dlg2).findByRole('button', { name: /as PDF/ }));
    await waitFor(() => expect(downloadReport).toHaveBeenCalledWith('pdf', expect.objectContaining({ id: db.reports[0].id }), expect.anything()));
    await user.click(within(dlg2).getByRole('button', { name: /^Delete/ }));
    const confirm = await screen.findByRole('dialog', { name: 'Delete report' });
    await user.click(within(confirm).getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(db.reports).toHaveLength(0));
  });
});
