// Client-side export entry points. Everything heavy (PDF, DOCX, BlockNote conversion) is a dynamic import,
// so only a download pays for it. The server stays JSON-only.
import type { ContentFormat, CycleBundle, Report } from '@/types/api/models';
import { downloadFile } from '@/lib/download';
import type { ExportDocument } from './document';
import { fileName } from './markdown';

export type ExportFormat = 'pdf' | 'docx' | 'markdown';

const MIME: Record<ExportFormat, string> = {
  markdown: 'text/markdown',
  docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  pdf: 'application/pdf',
};
const EXT: Record<ExportFormat, string> = { markdown: 'md', docx: 'docx', pdf: 'pdf' };

const today = () => new Date().toISOString().slice(0, 10);

/** Renders a document model to a PDF or DOCX blob. */
export async function renderDocument(doc: ExportDocument, format: 'pdf' | 'docx'): Promise<Blob> {
  const { localPageSize } = await import('./page');
  const pageSize = localPageSize(navigator.language);
  if (format === 'docx') return (await import('./docx')).documentToDocx(doc, pageSize);
  const [{ documentToPdf }, { PLEX_SANS_FILES }] = await Promise.all([import('./pdf'), import('./fonts')]);
  return documentToPdf(doc, pageSize, PLEX_SANS_FILES);
}

async function save(doc: ExportDocument, markdown: () => Promise<string>, format: ExportFormat) {
  const body: BlobPart = format === 'markdown' ? await markdown() : await renderDocument(doc, format);
  downloadFile(fileName(doc.title, today(), EXT[format]), body, MIME[format]);
}

type Section = { title: string; content: string; content_format: ContentFormat };

/** Markdown for a list of written sections; BlockNote content is converted, old markdown is kept as is. */
async function sectionsToMarkdown(sections: Section[], level: number, jiraBase?: string | null): Promise<string> {
  const { prepareBlocksForMarkdown } = await import('./markdown');
  const { contentToBlocks } = await import('./compose');
  const conv = await import('@/components/editor/convert');
  const out: string[] = [];
  for (const s of sections) {
    out.push(`${'#'.repeat(level)} ${s.title}\n`);
    if (s.content_format === 'markdown') out.push(conv.linkifyJiraKeys(s.content, jiraBase));
    else out.push(conv.blocksToMarkdown(prepareBlocksForMarkdown(await contentToBlocks(s.content, s.content_format, jiraBase))));
    out.push('');
  }
  return out.join('\n');
}

export async function downloadReport(format: ExportFormat, report: Report, jiraBase?: string | null): Promise<void> {
  const { reportToDocument, describeFilterEntries } = await import('./compose');
  const doc = await reportToDocument(report, jiraBase);
  const md = async () => {
    const s = report.stats;
    const lines = [`# ${report.title}`, '', `_${describeFilterEntries(report)} · generated ${report.generated_at.slice(0, 10)}_`, ''];
    if (report.narrative.content.trim()) lines.push(await sectionsToMarkdown([{ title: 'Summary', ...report.narrative }], 2, jiraBase));
    lines.push('## At a glance', '', `- Entries: ${s.entries}`, `- People: ${s.people}`, `- Repositories: ${s.repos}`, `- Teams: ${s.teams}`, '');
    const list = (title: string, items: { name?: string; label?: string; count: number }[]) => items.length ? [`## ${title}`, '', ...items.map((i) => `- ${i.name ?? i.label}: ${i.count}`), ''] : [];
    lines.push(...list('Entries per week', s.per_week.map((w) => ({ label: w.week_start, count: w.count }))), ...list('Projects', s.per_project), ...list('Who you worked with', s.people_top), ...list('Repositories', s.repos_top), ...list('Teams', s.teams_top));
    if (report.key_entries.length) lines.push('## Key entries', '', ...report.key_entries.map((e) => `- **${e.title}** (${e.occurred_at})${e.outcome_statement ? `: ${e.outcome_statement}` : ''}`), '');
    return lines.join('\n');
  };
  await save(doc, md, format);
}

export async function downloadReview(format: ExportFormat, title: string, sections: Section[], jiraBase?: string | null): Promise<void> {
  const { reviewToDocument } = await import('./compose');
  const doc = await reviewToDocument(title, sections, jiraBase);
  await save(doc, async () => `# ${title}\n\n${await sectionsToMarkdown(sections, 2, jiraBase)}`, format);
}

export async function downloadCycleBundle(format: ExportFormat | 'json', bundle: CycleBundle, jiraBase?: string | null): Promise<void> {
  if (format === 'json') {
    downloadFile(fileName(`${bundle.cycle.name} backup`, today(), 'json'), JSON.stringify(bundle, null, 2), 'application/json');
    return;
  }
  const { bundleToDocument } = await import('./compose');
  const doc = await bundleToDocument(bundle, jiraBase);
  const md = async () => {
    const lines = [`# ${bundle.cycle.name}`, '', `_${bundle.cycle.start_date} to ${bundle.cycle.end_date}_`, ''];
    if (bundle.review.length) lines.push('## Self review', '', await sectionsToMarkdown(bundle.review, 3, jiraBase));
    if (bundle.priorities.length) {
      lines.push('## Priorities', '');
      for (const pr of bundle.priorities) {
        lines.push(`### ${pr.title}`, '');
        for (const u of pr.updates) lines.push(`- ${u.created_at.slice(0, 10)}${u.is_blocker ? ' (blocker)' : ''}: ${u.comment}`);
        lines.push('');
      }
    }
    lines.push('## Entries', '');
    for (const w of bundle.weeks) {
      lines.push(`### Week ${w.iso_week} · ${w.week_start}`, '');
      for (const e of w.entries) lines.push(`- **${e.title}** · ${e.entry_type.replace(/_/g, ' ')} · ${e.occurred_at}`);
      lines.push('');
    }
    return lines.join('\n');
  };
  await save(doc, md, format);
}
