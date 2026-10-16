import { BlockNoteSchema, createCodeBlockSpec, defaultBlockSpecs, defaultInlineContentSpecs } from '@blocknote/core';
import { entryRefSpec } from './EntryRefSpec';

/** Default blocks plus a short code-language list; no drawings or media beyond BlockNote's defaults. */
export const bragSchema = BlockNoteSchema.create({
  blockSpecs: {
    ...defaultBlockSpecs,
    codeBlock: createCodeBlockSpec({
      defaultLanguage: 'text',
      supportedLanguages: {
        text: { name: 'Plain Text', aliases: ['plaintext', 'txt'] },
        javascript: { name: 'JavaScript', aliases: ['js'] },
        typescript: { name: 'TypeScript', aliases: ['ts'] },
        python: { name: 'Python', aliases: ['py'] },
        rust: { name: 'Rust', aliases: ['rs'] },
        json: { name: 'JSON' },
        bash: { name: 'Bash', aliases: ['sh', 'shell'] },
        sql: { name: 'SQL' },
      },
    }),
  },
  inlineContentSpecs: {
    ...defaultInlineContentSpecs,
    entryRef: entryRefSpec,
  },
});

export type BragSchema = typeof bragSchema;
