import type { PartialBlock } from '@blocknote/core';

export type EntryRefCandidate = { id: number; title: string; subtext?: string };

export type RichEditorHandle = {
  insertEntryRef: (c: EntryRefCandidate) => void;
  replaceAll: (blocks: PartialBlock[]) => void;
  insertBlocks: (blocks: PartialBlock[]) => void;
  getBlocks: () => PartialBlock[];
  isEmpty: () => boolean;
  flush: () => void;
};
