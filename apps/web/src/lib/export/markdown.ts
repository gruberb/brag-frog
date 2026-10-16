import type { PartialBlock } from '@blocknote/core';

interface LooseBlock {
  type?: string;
  content?: unknown;
  children?: LooseBlock[];
  [key: string]: unknown;
}
interface LooseInline {
  type?: string;
  props?: { label?: string };
}

/** BlockNote's Markdown export has no rule for entry references and would drop them; keep their title. */
function rewriteInline(content: unknown): unknown {
  if (!Array.isArray(content)) return content;
  return (content as LooseInline[]).map((c) => (c?.type === 'entryRef' ? { type: 'text', text: c.props?.label || 'Entry', styles: {} } : c));
}

function rewriteTable(content: { rows?: { cells?: unknown[] }[] }) {
  return {
    ...content,
    rows: (content.rows ?? []).map((row) => ({
      ...row,
      cells: (row.cells ?? []).map((cell) => (Array.isArray(cell) ? rewriteInline(cell) : { ...(cell as object), content: rewriteInline((cell as { content?: unknown })?.content) })),
    })),
  };
}

function prepare(block: LooseBlock): LooseBlock {
  const next: LooseBlock = { ...block };
  if (Array.isArray(block.content)) next.content = rewriteInline(block.content);
  else if (block.type === 'table' && block.content) next.content = rewriteTable(block.content as { rows?: { cells?: unknown[] }[] });
  if (block.children?.length) next.children = block.children.map(prepare);
  return next;
}

export const prepareBlocksForMarkdown = (blocks: PartialBlock[]): PartialBlock[] => (blocks as LooseBlock[]).map(prepare) as PartialBlock[];

export const slug = (s: string) =>
  s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 60) || 'document';

export const fileName = (title: string, date: string, ext: string) => `${slug(title)}-${date}.${ext}`;
