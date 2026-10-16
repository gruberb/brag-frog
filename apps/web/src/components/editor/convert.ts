import { BlockNoteEditor, type PartialBlock } from '@blocknote/core';
import type { ContentFormat } from '@/types/api/models';
import { bragSchema } from './schema';

// A headless editor with the app schema, used only to convert; it is never mounted.
let headless: BlockNoteEditor<never, never, never> | null = null;
const getEditor = () => (headless ??= BlockNoteEditor.create({ schema: bragSchema }) as unknown as BlockNoteEditor<never, never, never>);

const JIRA_KEY = /(?<![\w/[(`])([A-Z][A-Z0-9]{1,9}-\d{1,6})(?![\w\])`])/g;

/** Turns bare `PROJ-123` into markdown links. Jira links are a client concern since the move to the SPA. */
export function linkifyJiraKeys(markdown: string, baseUrl: string | null | undefined): string {
  if (!baseUrl) return markdown;
  const base = baseUrl.replace(/\/$/, '');
  return markdown.replace(JIRA_KEY, (key) => `[${key}](${base}/browse/${key})`);
}

export async function markdownToBlocks(markdown: string, jiraBase?: string | null): Promise<PartialBlock[]> {
  return (await getEditor().tryParseMarkdownToBlocks(linkifyJiraKeys(markdown, jiraBase))) as PartialBlock[];
}

export function blocksToMarkdown(blocks: PartialBlock[]): string {
  return getEditor().blocksToMarkdownLossy(blocks as never);
}

/** Parses stored content in either format. Invalid blocknote JSON falls back to an empty document. */
export async function contentToBlocks(content: string, format: ContentFormat, jiraBase?: string | null): Promise<PartialBlock[]> {
  if (!content.trim()) return [];
  if (format === 'markdown') return markdownToBlocks(content, jiraBase);
  try {
    const parsed = JSON.parse(content);
    return Array.isArray(parsed) ? (parsed as PartialBlock[]) : [];
  } catch {
    return markdownToBlocks(content, jiraBase);
  }
}
