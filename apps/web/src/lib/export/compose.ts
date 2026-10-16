// Builds ExportDocument values (one model for PDF, DOCX and Markdown) from API payloads.
import type { PartialBlock } from '@blocknote/core';
import type { ContentFormat, CycleBundle, Entry, Priority, PriorityUpdate, Report } from '@/types/api/models';
import { bars, blocksToDocBlocks, bullet, h, p, pagebreak, table, type DocBlock, type ExportDocument } from './document';

const day = (iso: string) => iso.slice(0, 10);

/** Parses stored content in either format. The editor conversion lives in its own lazy chunk. */
export async function contentToBlocks(content: string, format: ContentFormat, jiraBase?: string | null): Promise<PartialBlock[]> {
  const m = await import('@/components/editor/convert');
  return m.contentToBlocks(content, format, jiraBase);
}

async function contentToDoc(content: string, format: ContentFormat, jiraBase: string | null | undefined, listOffset: number): Promise<DocBlock[]> {
  if (!content.trim()) return [];
  return blocksToDocBlocks(await contentToBlocks(content, format, jiraBase), listOffset);
}

export function describeFilterEntries(r: Pick<Report, 'filters' | 'entry_count'>): string {
  const f = r.filters;
  const parts = [f.from || f.to ? `${f.from ?? 'start'} to ${f.to ?? 'today'}` : 'Whole cycle', f.q && `search “${f.q}”`, f.source, f.category, f.team && `team ${f.team}`, f.person && `with ${f.person}`].filter(Boolean);
  return `${parts.join(' · ')} · ${r.entry_count} entries`;
}

export async function reportToDocument(r: Report, jiraBase?: string | null): Promise<ExportDocument> {
  const s = r.stats;
  const blocks: DocBlock[] = [];
  if (r.narrative.content.trim()) blocks.push(h(1, 'Summary'), ...(await contentToDoc(r.narrative.content, r.narrative.content_format as ContentFormat, jiraBase, 0)));
  blocks.push(h(1, 'At a glance'), table([['Entries', 'People', 'Repositories', 'Teams'], [String(s.entries), String(s.people), String(s.repos), String(s.teams)]]));
  if (s.per_week.length) blocks.push(h(2, 'Entries per week'), bars(s.per_week.map((w) => ({ label: w.week_start, value: w.count }))));
  if (s.per_project.length) blocks.push(h(2, 'Projects'), bars(s.per_project.map((x) => ({ label: x.name, value: x.count }))));
  if (s.people_top.length) blocks.push(h(2, 'Who you worked with'), bars(s.people_top.map((x) => ({ label: x.name, value: x.count }))));
  if (s.repos_top.length) blocks.push(h(2, 'Repositories'), bars(s.repos_top.map((x) => ({ label: x.name, value: x.count }))));
  if (s.teams_top.length) blocks.push(h(2, 'Teams'), bars(s.teams_top.map((x) => ({ label: x.name, value: x.count }))));
  if (r.key_entries.length) {
    blocks.push(h(1, 'Key entries'));
    for (const e of r.key_entries) blocks.push(bullet([{ text: `${e.occurred_at} · ${e.title}`, bold: true }, { text: ` · ${e.entry_type.replace(/_/g, ' ')}${e.repository ? ` · ${e.repository}` : ''}` }]));
  }
  return { title: r.title, meta: `${describeFilterEntries(r)} · generated ${day(r.generated_at)}`, blocks };
}

export async function reviewToDocument(title: string, sections: { title: string; content: string; content_format: ContentFormat }[], jiraBase?: string | null): Promise<ExportDocument> {
  const blocks: DocBlock[] = [];
  let lists = 0;
  for (const s of sections) {
    blocks.push(h(1, s.title));
    const doc = await contentToDoc(s.content, s.content_format, jiraBase, lists);
    lists += doc.length;
    blocks.push(...doc);
  }
  return { title, meta: `Exported ${day(new Date().toISOString())}`, blocks };
}

const TRACK: Record<string, string> = { on_track: 'On track', progressing: 'Progressing', off_track: 'At risk', complete: 'Completed', incomplete: 'Incomplete', no_update: 'No update' };

function priorityBlocks(pr: Priority, updates: PriorityUpdate[]): DocBlock[] {
  const out: DocBlock[] = [h(3, pr.title), p([TRACK[pr.tracking_status ?? 'no_update'] ?? pr.tracking_status, pr.due_date && `due ${pr.due_date}`, pr.status].filter(Boolean).join(' · '), { italic: true })];
  for (const u of [...updates].sort((a, b) => (a.created_at < b.created_at ? 1 : -1))) out.push(bullet(`${day(u.created_at)}${u.is_blocker ? ' (blocker)' : ''}: ${u.comment ?? ''}`));
  return out;
}

const entryLine = (e: Entry) => bullet([{ text: e.title, bold: true }, { text: ` · ${e.entry_type.replace(/_/g, ' ')} · ${e.occurred_at}` }]);

/** One document for a whole cycle: written review, priorities with updates, entries grouped by ISO week (page break per week). */
export async function bundleToDocument(b: CycleBundle, jiraBase?: string | null): Promise<ExportDocument> {
  const blocks: DocBlock[] = [];
  const written = b.review.sections.filter((s) => s.content?.trim());
  if (written.length) {
    blocks.push(h(1, 'Self review'));
    let lists = 0;
    for (const s of written) {
      blocks.push(h(2, s.title));
      const doc = await contentToDoc(s.content ?? '', s.content_format as ContentFormat, jiraBase, lists);
      lists += doc.length;
      blocks.push(...doc);
    }
  }
  const updatesOf = (id: number) => b.timelines.find((t) => t.priority_id === id)?.updates ?? [];
  if (b.priorities.goals.length || b.priorities.unassigned.length) {
    blocks.push(pagebreak(), h(1, 'Goals and priorities'));
    for (const g of b.priorities.goals) {
      blocks.push(h(2, g.goal.title));
      for (const pr of g.priorities) blocks.push(...priorityBlocks(pr, updatesOf(pr.id)));
    }
    if (b.priorities.unassigned.length) {
      blocks.push(h(2, 'No goal'));
      for (const pr of b.priorities.unassigned) blocks.push(...priorityBlocks(pr, updatesOf(pr.id)));
    }
  }
  const total = b.weeks.reduce((n, w) => n + w.entries.length, 0);
  blocks.push(pagebreak(), h(1, `Entries (${total})`));
  b.weeks.forEach((w, i) => {
    if (i > 0) blocks.push(pagebreak());
    blocks.push(h(2, `Week ${w.iso_week} · ${w.start_date}`));
    for (const e of w.entries) blocks.push(entryLine(e));
  });
  return { title: `${b.cycle.name} bundle`, meta: `${b.cycle.start_date} to ${b.cycle.end_date} · ${total} entries`, blocks };
}
