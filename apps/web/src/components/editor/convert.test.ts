import { describe, expect, it } from 'vitest';
import { linkifyJiraKeys, markdownToBlocks } from './convert';

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
});
