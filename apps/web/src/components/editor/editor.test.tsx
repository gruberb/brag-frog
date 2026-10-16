import { createRef } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { LazyRichEditor } from './LazyRichEditor';
import type { RichEditorHandle } from './types';
import { linkifyJiraKeys, markdownToBlocks } from './convert';

describe('RichEditor', () => {
  it('inserts an entry reference chip, autosaves it, and opens the entry on click', async () => {
    const user = userEvent.setup();
    const handle = createRef<RichEditorHandle>();
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onOpenEntry = vi.fn();
    render(<LazyRichEditor ref={handle} docKey="t" initialBlocks={[]} onSave={onSave} onOpenEntry={onOpenEntry} />);
    await waitFor(() => expect(handle.current).not.toBeNull(), { timeout: 8000 });

    act(() => handle.current!.insertEntryRef({ id: 7, title: 'Merged autoscaling' }));
    const chip = await screen.findByText('Merged autoscaling');
    expect(chip).toHaveAttribute('data-entry-ref', '7');

    // Autosave fires after the 500 ms debounce and stores the reference as inline content.
    await waitFor(() => expect(onSave).toHaveBeenCalled(), { timeout: 10000 });
    expect(JSON.stringify(onSave.mock.calls.at(-1)![0])).toContain('"type":"entryRef"');

    await user.click(chip);
    expect(onOpenEntry).toHaveBeenCalledWith(7);
  }, 40000);

  it('does not echo an external replace back as a user save, but saves AI drafts', async () => {
    const handle = createRef<RichEditorHandle>();
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(<LazyRichEditor ref={handle} docKey="t2" initialBlocks={[]} onSave={onSave} />);
    await waitFor(() => expect(handle.current).not.toBeNull(), { timeout: 8000 });
    expect(handle.current!.isEmpty()).toBe(true);
    act(() => handle.current!.replaceAll([{ type: 'paragraph', content: 'Drafted text' }]));
    expect(handle.current!.isEmpty()).toBe(false);
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1), { timeout: 10000 });
  }, 40000);
});

describe('convert', () => {
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
