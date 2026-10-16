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
