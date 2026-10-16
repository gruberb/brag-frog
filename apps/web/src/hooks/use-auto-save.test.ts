import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAutoSave } from './use-auto-save';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('useAutoSave', () => {
  it('saves once, 500 ms after the last change', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useAutoSave<string>(save));
    act(() => result.current.schedule('a'));
    act(() => { vi.advanceTimersByTime(300); });
    act(() => result.current.schedule('ab'));
    act(() => { vi.advanceTimersByTime(499); });
    expect(save).not.toHaveBeenCalled();
    expect(result.current.isPending()).toBe(true);
    await act(async () => { vi.advanceTimersByTime(1); });
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith('ab');
  });

  it('flushes the pending save on unmount', () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const { result, unmount } = renderHook(() => useAutoSave<string>(save));
    act(() => result.current.schedule('draft'));
    unmount();
    expect(save).toHaveBeenCalledWith('draft');
  });

  it('flushes when the tab is hidden and on pagehide', () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useAutoSave<string>(save));
    act(() => result.current.schedule('one'));
    Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true });
    act(() => { document.dispatchEvent(new Event('visibilitychange')); });
    expect(save).toHaveBeenCalledWith('one');
    act(() => result.current.schedule('two'));
    act(() => { window.dispatchEvent(new Event('pagehide')); });
    expect(save).toHaveBeenLastCalledWith('two');
    Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true });
  });

  it('reports failures without throwing and keeps working', async () => {
    const onError = vi.fn();
    const save = vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValue(undefined);
    const { result } = renderHook(() => useAutoSave<string>(save, { onError }));
    act(() => result.current.schedule('x'));
    await act(async () => { vi.advanceTimersByTime(500); });
    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ message: 'offline' }));
    act(() => result.current.schedule('y'));
    await act(async () => { vi.advanceTimersByTime(500); });
    expect(save).toHaveBeenLastCalledWith('y');
  });
});
