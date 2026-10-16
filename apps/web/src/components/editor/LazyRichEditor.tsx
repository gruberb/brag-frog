import { forwardRef, lazy, Suspense, type ComponentProps } from 'react';
import { Skeleton } from '@/components/ui/Spinner';
import type { RichEditorHandle } from './types';

// BlockNote and Mantine are only fetched when an editor is rendered (the Review route).
const RichEditor = lazy(() => import('./RichEditor'));

export const LazyRichEditor = forwardRef<RichEditorHandle, ComponentProps<typeof RichEditor>>(function LazyRichEditor(props, ref) {
  return (
    <Suspense fallback={<div className="editor-host-skeleton col"><Skeleton h={28} w="60%" /><Skeleton h={16} /><Skeleton h={16} /><Skeleton h={16} w="80%" /></div>}>
      <RichEditor ref={ref} {...props} />
    </Suspense>
  );
});
