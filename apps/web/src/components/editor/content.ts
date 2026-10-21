import type { ContentFormat } from '@/types/api/models';

/** True when any inline text in a BlockNote tree has visible characters. */
function blocksHaveText(node: unknown): boolean {
  if (Array.isArray(node)) return node.some(blocksHaveText);
  if (!node || typeof node !== 'object') return false;
  const n = node as { text?: unknown; content?: unknown; children?: unknown; props?: { label?: unknown } };
  if (typeof n.text === 'string' && n.text.trim()) return true;
  if (typeof n.props?.label === 'string' && n.props.label.trim()) return true; // entry reference chips
  return blocksHaveText(n.content) || blocksHaveText(n.children);
}

/**
 * Whether saved section text has anything in it. An editor that was opened and cleared still
 * saves a document with an empty paragraph, so a plain string check is not enough. Kept free
 * of BlockNote so pages can call it without loading the editor chunk.
 */
export function hasContent(content: string | null | undefined, format: ContentFormat | string): boolean {
  if (!content?.trim()) return false;
  if (format !== 'blocknote') return true;
  try {
    return blocksHaveText(JSON.parse(content));
  } catch {
    return true;
  }
}

type BlockNode = { text?: unknown; content?: unknown; children?: unknown; props?: { label?: unknown } };

/** Visible text of one block's inline content: text runs, link text and entry chip labels. */
function inlineText(node: unknown): string {
  if (Array.isArray(node)) return node.map(inlineText).join('');
  if (!node || typeof node !== 'object') return '';
  const n = node as BlockNode;
  if (typeof n.text === 'string') return n.text;
  if (typeof n.props?.label === 'string') return n.props.label;
  return inlineText(n.content);
}

/** The normalized text of every block in a tree (nested blocks included), empty ones dropped. */
function blockTexts(blocks: unknown, out: string[] = []): string[] {
  if (!Array.isArray(blocks)) return out;
  for (const b of blocks as BlockNode[]) {
    const text = inlineText(b?.content).replace(/\s+/g, ' ').trim();
    if (text) out.push(text);
    blockTexts(b?.children, out);
  }
  return out;
}

/**
 * True when every non-empty block in `doc` is a line of `outline`: the document holds the
 * section's outline (once or several times, maybe partly deleted) and nothing the user wrote.
 * An AI draft may then replace it instead of being appended below it.
 */
export function onlyOutline(doc: unknown[], outline: unknown[]): boolean {
  const lines = new Set(blockTexts(outline));
  return blockTexts(doc).every((t) => lines.has(t));
}
