import type { PartialBlock } from '@blocknote/core';

export type EntryRefCandidate = { id: number; title: string; subtext?: string };

/** An evidence item as it is written into the document: readable text, linked to its source. */
export type EvidenceItem = { title: string; url?: string | null; meta?: string };

export type RichEditorHandle = {
  insertEntryRef: (c: EntryRefCandidate) => void;
  insertEvidence: (item: EvidenceItem) => void;
  replaceAll: (blocks: PartialBlock[]) => void;
  insertBlocks: (blocks: PartialBlock[]) => void;
  getBlocks: () => PartialBlock[];
  isEmpty: () => boolean;
  flush: () => void;
};
