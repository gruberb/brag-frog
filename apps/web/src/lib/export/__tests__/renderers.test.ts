// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { fileURLToPath } from 'node:url';
import type { DocBlock, ExportDocument } from '../document';
import { documentToDocx } from '../docx';
import { documentToPdf } from '../pdf';
import type { PdfFontFiles } from '../fonts';

// The Node build of the renderer reads font files from disk.
const woff = (face: string) => fileURLToPath(new URL(`../../../../../../node_modules/@ibm/plex-sans/fonts/complete/woff/IBMPlexSans-${face}.woff`, import.meta.url));
const fonts: PdfFontFiles = { regular: woff('Regular'), bold: woff('Bold'), italic: woff('Italic'), boldItalic: woff('BoldItalic') };

// Every block type, nesting, page breaks, bars and characters outside the PDF fonts.
const doc: ExportDocument = {
  title: 'Q4 report',
  meta: '2026-10-15  ·  87 entries',
  blocks: [
    { type: 'heading', depth: 0, level: 1, runs: [{ text: 'Summary → plan' }] },
    { type: 'paragraph', depth: 0, runs: [{ text: 'bold', bold: true }, { text: ' link', href: 'https://example.com' }, { text: ' code', code: true }] },
    { type: 'paragraph', depth: 0, runs: [] },
    { type: 'quote', depth: 0, runs: [{ text: 'quoted 日本語 🙂' }] },
    { type: 'bullet', depth: 0, runs: [{ text: 'bullet' }] },
    { type: 'number', depth: 1, number: 3, list: 0, runs: [{ text: 'nested number' }] },
    { type: 'check', depth: 0, checked: true, runs: [{ text: 'done' }] },
    { type: 'code', depth: 0, text: 'fn main() {\n    println!("hi");\n}' },
    { type: 'table', depth: 0, headerRows: 1, rows: [[[{ text: 'A' }], [{ text: 'B' }]], [[{ text: 'only one cell' }]]] },
    { type: 'divider', depth: 0 },
    { type: 'bars', depth: 0, items: [{ label: 'ci-tools', value: 92 }, { label: 'workspace', value: 61 }] },
    { type: 'pagebreak', depth: 0 },
    { type: 'heading', depth: 0, level: 2, runs: [{ text: 'Week 42' }] },
  ],
};

const bytes = async (blob: Blob) => new TextDecoder('latin1').decode(await blob.arrayBuffer());

describe('export renderers', () => {
  it('writes a Word document', async () => {
    const out = await bytes(await documentToDocx(doc, 'A4'));
    expect(out.startsWith('PK')).toBe(true);
    expect(out).toContain('word/document.xml');
  });

  it('writes a PDF', async () => {
    const out = await bytes(await documentToPdf(doc, 'LETTER', fonts));
    expect(out.startsWith('%PDF-')).toBe(true);
  }, 30_000);

  it('keeps a 400-entry document readable (one page break per week, no failure)', async () => {
    const big: ExportDocument = {
      title: 'Cycle',
      meta: 'bundle',
      blocks: Array.from({ length: 20 }, (_, w): DocBlock[] => [
        ...(w ? [{ type: 'pagebreak', depth: 0 } satisfies DocBlock] : []),
        { type: 'heading', depth: 0, level: 2, runs: [{ text: `Week ${w}` }] },
        ...Array.from({ length: 20 }, (_, i): DocBlock => ({ type: 'bullet', depth: 0, runs: [{ text: `Entry ${w}-${i} with a fairly long title that may wrap onto a second line in the page` }] })),
      ]).flat(),
    };
    const out = await bytes(await documentToPdf(big, 'A4', fonts));
    expect(out.startsWith('%PDF-')).toBe(true);
    expect((out.match(/\/Type \/Page\b/g) ?? []).length).toBeGreaterThanOrEqual(20);
  }, 60_000);
});
