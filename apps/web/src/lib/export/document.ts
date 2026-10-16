// Format-neutral document model for the PDF, Word and Markdown exporters (adapted from workledger).
// BlockNote's tree is flattened into blocks with a nesting depth, list numbers are resolved and
// links are checked here, so each renderer only maps a small, closed set of block types.

export interface TextRun {
  text: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  code?: boolean;
  /** Only http, https and mailto: values arrive through sync and import. */
  href?: string;
}

export type DocBlock = { depth: number } & (
  | { type: 'heading'; level: 1 | 2 | 3 | 4 | 5 | 6; runs: TextRun[] }
  | { type: 'paragraph'; runs: TextRun[] }
  | { type: 'quote'; runs: TextRun[] }
  | { type: 'bullet'; runs: TextRun[] }
  /** `list` is shared by consecutive siblings, so renderers can restart numbering per list. */
  | { type: 'number'; number: number; list: number; runs: TextRun[] }
  | { type: 'check'; checked: boolean; runs: TextRun[] }
  | { type: 'code'; text: string }
  | { type: 'table'; rows: TextRun[][][]; headerRows: number }
  | { type: 'divider' }
  /** Horizontal bars for small rankings; Word falls back to a table with a text bar. */
  | { type: 'bars'; items: { label: string; value: number }[] }
  | { type: 'pagebreak' }
);

export interface ExportDocument {
  title: string;
  /** Period, filters and generation date shown above the body. */
  meta: string;
  blocks: DocBlock[];
}

interface LooseBlock {
  type?: string;
  props?: Record<string, unknown>;
  content?: unknown;
  children?: LooseBlock[];
}

interface LooseInline {
  type?: string;
  text?: string;
  href?: string;
  styles?: Record<string, unknown>;
  content?: unknown;
  props?: { label?: string };
}

const SAFE_LINK = /^(https?:|mailto:)/i;
const safeHref = (href: unknown) => (typeof href === 'string' && SAFE_LINK.test(href.trim()) ? href.trim() : undefined);
const str = (v: unknown) => (typeof v === 'string' ? v : '');

function styledRun(text: string, styles: Record<string, unknown> = {}, href?: string): TextRun {
  const run: TextRun = { text };
  if (styles.bold) run.bold = true;
  if (styles.italic) run.italic = true;
  if (styles.underline) run.underline = true;
  if (styles.strike) run.strike = true;
  if (styles.code) run.code = true;
  if (href) run.href = href;
  return run;
}

export function toRuns(content: unknown): TextRun[] {
  if (typeof content === 'string') return content ? [{ text: content }] : [];
  if (!Array.isArray(content)) return [];
  const runs: TextRun[] = [];
  for (const item of content as (LooseInline | string)[]) {
    if (typeof item === 'string') {
      if (item) runs.push({ text: item });
    } else if (item?.type === 'text' && item.text) {
      runs.push(styledRun(item.text, item.styles));
    } else if (item?.type === 'link') {
      const href = safeHref(item.href);
      for (const inner of toRuns(item.content)) runs.push(href ? { ...inner, href } : inner);
    } else if (item?.type === 'entryRef') {
      // The entry may not exist outside the app, so the reference keeps only its title.
      runs.push({ text: item.props?.label || 'Entry' });
    }
  }
  return runs;
}

function tableRows(content: unknown) {
  const table = (content ?? {}) as { rows?: { cells?: unknown[] }[]; headerRows?: number };
  const rows = (table.rows ?? []).map((row) =>
    (row.cells ?? []).map((cell) => (cell && typeof cell === 'object' && !Array.isArray(cell) ? toRuns((cell as { content?: unknown }).content) : toRuns(cell))),
  );
  return { rows, headerRows: typeof table.headerRows === 'number' ? table.headerRows : 0 };
}

const headingLevel = (v: unknown): 1 | 2 | 3 | 4 | 5 | 6 => {
  const n = Number(v);
  return n >= 1 && n <= 6 ? (n as 1 | 2 | 3 | 4 | 5 | 6) : 1;
};

function mapBlock(block: LooseBlock, props: Record<string, unknown>, depth: number): DocBlock | null {
  const runs = () => toRuns(block.content);
  switch (block.type) {
    case 'heading':
      return { type: 'heading', depth, level: headingLevel(props.level), runs: runs() };
    case 'quote':
      return { type: 'quote', depth, runs: runs() };
    // Toggle lists export expanded, since paper cannot collapse.
    case 'bulletListItem':
    case 'toggleListItem':
      return { type: 'bullet', depth, runs: runs() };
    case 'checkListItem':
      return { type: 'check', depth, checked: props.checked === true, runs: runs() };
    case 'codeBlock':
      return { type: 'code', depth, text: toRuns(block.content).map((r) => r.text).join('') };
    case 'table':
      return { type: 'table', depth, ...tableRows(block.content) };
    case 'divider':
      return { type: 'divider', depth };
    // Media cannot play on paper and images are not embedded; keep a link so nothing silently disappears.
    case 'image':
    case 'file':
    case 'video':
    case 'audio': {
      const href = safeHref(props.url);
      const label = str(props.caption) || str(props.name) || str(props.url) || block.type || 'file';
      return { type: 'paragraph', depth, runs: [{ text: `${(block.type ?? 'file')[0].toUpperCase()}${(block.type ?? 'file').slice(1)}: ` }, { text: label, href }] };
    }
    default:
      return { type: 'paragraph', depth, runs: runs() };
  }
}

function flatten(blocks: LooseBlock[], depth: number, out: DocBlock[], nextList: () => number) {
  let list = -1;
  let number = 0;
  for (const block of blocks) {
    const props = block.props ?? {};
    if (block.type === 'numberedListItem') {
      if (list < 0) {
        list = nextList();
        const start = Number(props.start);
        number = Number.isInteger(start) && start > 0 ? start : 1;
      } else number += 1;
      out.push({ type: 'number', depth, number, list, runs: toRuns(block.content) });
    } else {
      list = -1;
      const mapped = mapBlock(block, props, depth);
      if (mapped) out.push(mapped);
    }
    if (block.children?.length) flatten(block.children, depth + 1, out, nextList);
  }
}

/** Flattens BlockNote blocks. `listOffset` keeps numbering lists distinct when several documents are joined. */
export function blocksToDocBlocks(blocks: unknown[], listOffset = 0): DocBlock[] {
  let lists = listOffset;
  const out: DocBlock[] = [];
  flatten(blocks as LooseBlock[], 0, out, () => lists++);
  return out;
}

// Small constructors keep the composers readable.
export const h = (level: 1 | 2 | 3, text: string): DocBlock => ({ type: 'heading', depth: 0, level, runs: [{ text }] });
export const p = (text: string, opts: Partial<TextRun> = {}): DocBlock => ({ type: 'paragraph', depth: 0, runs: text ? [{ text, ...opts }] : [] });
export const bullet = (runs: TextRun[] | string): DocBlock => ({ type: 'bullet', depth: 0, runs: typeof runs === 'string' ? [{ text: runs }] : runs });
export const pagebreak = (): DocBlock => ({ type: 'pagebreak', depth: 0 });
export const bars = (items: { label: string; value: number }[]): DocBlock => ({ type: 'bars', depth: 0, items });
export const table = (rows: string[][], headerRows = 1): DocBlock => ({ type: 'table', depth: 0, headerRows, rows: rows.map((r) => r.map((c) => [{ text: c }])) });
