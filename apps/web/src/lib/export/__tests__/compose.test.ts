import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CycleBundle, Report } from '@/types/api/models';
import type { EntryDto } from '@/types/api/EntryDto';
import type { PriorityDto } from '@/types/api/PriorityDto';
import { bundleToDocument, reportToDocument } from '../compose';
import { blocksToDocBlocks } from '../document';
import { prepareBlocksForMarkdown } from '../markdown';
import { downloadCycleBundle, downloadReport } from '../index';

const entry = (id: number, title: string, occurred_at: string) => ({ id, title, occurred_at, entry_type: 'pr_merged' }) as unknown as EntryDto;

const report: Report = {
  id: 1, cycle_id: 2, title: 'Reduce CI time · Q4', filters: { priority_id: 11, from: '2026-10-01' }, entry_count: 2, generated_at: '2026-10-16T10:00:00Z', updated_at: '2026-10-16T10:00:00Z', model_used: null, ai_error: null,
  stats: {
    entries: 2, people: 1, repos: 1, teams: 1, per_week: [{ week_start: '2026-10-12', count: 2 }], per_project: [{ priority_id: 11, name: 'Reduce CI time', count: 2 }],
    people_top: [{ name: 'Sam', count: 2 }], repos_top: [{ name: 'ci-tools', count: 2 }], teams_top: [{ name: 'Platform', count: 2 }],
  },
  narrative: { content: '', content_format: 'markdown' },
  key_entries: [{ id: 1, title: 'Merged autoscaling', entry_type: 'pr_merged', source: 'github', source_url: null, occurred_at: '2026-10-15', priority_id: 11, repository: 'ci-tools', reach: null, complexity: null }],
};

const reduceCi: PriorityDto = {
  id: 11, cycle_id: 2, title: 'Reduce CI', status: 'active', color: null, sort_order: 1, scope: null, started_at: null, completed_at: null, impact_narrative: null, department_goal_id: 1, created_at: '2026-07-01T00:00:00Z',
  priority_level: null, measure_type: null, measure_start: null, measure_target: null, measure_current: null, description: null, tracking_status: 'off_track', due_date: null, tier: null, entry_count: 3, latest_update: null,
};

const bundle: CycleBundle = {
  cycle: { id: 2, name: 'H2 2026', start_date: '2026-07-01', end_date: '2026-12-31', is_active: true },
  exported_at: '2026-10-16T10:00:00Z',
  review: { cycle_id: 2, sections: [], entry_count: 3 },
  priorities: {
    cycle_id: 2, unassigned: [], counts: { total: 1, active: 1, not_started: 0, on_hold: 0, completed: 0, cancelled: 0 },
    goals: [{ goal: { id: 1, cycle_id: 2, title: 'Velocity', description: null, status: 'active', sort_order: 1, source: 'manual', created_at: '2026-07-01T00:00:00Z' }, priorities: [reduceCi] }],
  },
  timelines: [{ priority_id: 11, updates: [{ id: 1, priority_id: 11, tracking_status: 'off_track', measure_value: null, comment: 'Blocked', is_blocker: true, tradeoff_text: null, created_at: '2026-10-14T00:00:00Z' }] }],
  weeks: [
    { iso_week: 41, year: 2026, start_date: '2026-10-05', end_date: '2026-10-11', entries: [entry(1, 'A', '2026-10-06')] },
    { iso_week: 42, year: 2026, start_date: '2026-10-12', end_date: '2026-10-18', entries: [entry(2, 'B', '2026-10-13'), entry(3, 'C', '2026-10-14')] },
  ],
};

afterEach(() => vi.restoreAllMocks());

// jsdom's Blob has no text(); FileReader is available.
const readText = (b: Blob) => new Promise<string>((res, rej) => { const r = new FileReader(); r.onload = () => res(String(r.result)); r.onerror = rej; r.readAsText(b); });

describe('compose', () => {
  it('builds a report with stats, rankings and key entries', async () => {
    const doc = await reportToDocument(report);
    expect(doc.title).toBe('Reduce CI time · Q4');
    expect(doc.meta).toContain('2 entries');
    const headings = doc.blocks.filter((b) => b.type === 'heading').map((b) => b.type === 'heading' && b.runs[0].text);
    expect(headings).toEqual(['At a glance', 'Entries per week', 'Projects', 'Who you worked with', 'Repositories', 'Teams', 'Key entries']);
    expect(doc.blocks.some((b) => b.type === 'bars')).toBe(true);
  });

  it('builds a cycle bundle with a page break per week', async () => {
    const doc = await bundleToDocument(bundle);
    expect(doc.blocks.filter((b) => b.type === 'pagebreak')).toHaveLength(3); // before priorities, before entries, between the two weeks
    const weekHeads = doc.blocks.filter((b) => b.type === 'heading' && b.level === 2 && /^Week/.test(b.runs[0].text));
    expect(weekHeads).toHaveLength(2);
    expect(doc.blocks.filter((b) => b.type === 'bullet').length).toBeGreaterThanOrEqual(4);
  });

  it('flattens blocknote blocks and keeps entryRef titles', () => {
    const blocks = blocksToDocBlocks([
      { type: 'numberedListItem', content: [{ type: 'text', text: 'one', styles: {} }] },
      { type: 'numberedListItem', content: [{ type: 'text', text: 'two', styles: {} }, { type: 'entryRef', props: { entryId: '7', label: 'Merged autoscaling' } }] },
      { type: 'paragraph', content: [{ type: 'link', href: 'javascript:alert(1)', content: [{ type: 'text', text: 'bad', styles: {} }] }] },
    ]);
    expect(blocks[1]).toMatchObject({ type: 'number', number: 2 });
    expect(blocks[1].type === 'number' && blocks[1].runs.map((r) => r.text)).toEqual(['two', 'Merged autoscaling']);
    expect(blocks[2].type === 'paragraph' && blocks[2].runs[0].href).toBeUndefined();
  });

  it('rewrites entryRef to plain text for Markdown', () => {
    const out = prepareBlocksForMarkdown([{ type: 'paragraph', content: [{ type: 'entryRef', props: { entryId: '7', label: 'Merged autoscaling' } }] } as never]);
    expect(JSON.stringify(out)).toContain('Merged autoscaling');
    expect(JSON.stringify(out)).not.toContain('entryRef');
  });
});

describe('downloads', () => {
  const capture = () => {
    const blobs: Blob[] = [];
    const names: string[] = [];
    URL.createObjectURL = (b: Blob | MediaSource) => (blobs.push(b as Blob), 'blob:x');
    URL.revokeObjectURL = () => {};
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) { names.push(this.download); });
    return { blobs, names };
  };

  it('downloads a report as Markdown without loading the PDF stack', async () => {
    const { blobs, names } = capture();
    await downloadReport('markdown', report);
    expect(names[0]).toMatch(/^reduce-ci-time-q4-\d{4}-\d{2}-\d{2}\.md$/);
    const text = await readText(blobs[0]);
    expect(text).toContain('# Reduce CI time · Q4');
    expect(text).toContain('- Sam: 2');
  });

  it('downloads the raw JSON backup of a cycle', async () => {
    const { blobs, names } = capture();
    await downloadCycleBundle('json', bundle);
    expect(names[0]).toMatch(/\.json$/);
    expect(JSON.parse(await readText(blobs[0])).cycle.name).toBe('H2 2026');
  });

  it('downloads a cycle bundle as Markdown', async () => {
    const { blobs } = capture();
    await downloadCycleBundle('markdown', bundle);
    const text = await readText(blobs[0]);
    expect(text).toContain('### Week 42');
    expect(text).toContain('(blocker): Blocked');
  });
});
