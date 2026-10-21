import { describe, expect, it } from 'vitest';
import { linkifyJiraKeys, markdownToBlocks } from './convert';
import { onlyOutline } from './content';

describe('editor conversion', () => {
  it('links bare Jira keys but not ones already in links or code', () => {
    const out = linkifyJiraKeys('See PLAT-12, [PLAT-13](http://x) and `PLAT-14`.', 'https://x.atlassian.net/');
    expect(out).toContain('[PLAT-12](https://x.atlassian.net/browse/PLAT-12)');
    expect(out).toContain('[PLAT-13](http://x)');
    expect(out).toContain('`PLAT-14`');
    expect(linkifyJiraKeys('PLAT-1', null)).toBe('PLAT-1');
  });

  it('converts markdown to blocks', async () => {
    const blocks = await markdownToBlocks('## Title\n\nBody');
    expect(blocks[0]).toMatchObject({ type: 'heading' });
  });

  it('tells an untouched (or repeated) outline from a document with writing in it', async () => {
    const outline = await markdownToBlocks('## Example 1: <headline>\n\n### Outcome\n_What changed._\n- \n\n### CLG\n- Complexity: \n- Risk management: ');
    expect(onlyOutline(outline, outline)).toBe(true);
    expect(onlyOutline([...outline, ...outline], outline)).toBe(true);
    expect(onlyOutline([], outline)).toBe(true);
    const written = await markdownToBlocks('## Example 1: <headline>\n\n### Outcome\n- Cut CI time by 45%');
    expect(onlyOutline(written, outline)).toBe(false);
    const labelFilled = await markdownToBlocks('- Complexity: ran spikes on three build systems');
    expect(onlyOutline(labelFilled, outline)).toBe(false);
  });
});
