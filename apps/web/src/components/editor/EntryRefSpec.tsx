import { createReactInlineContentSpec } from '@blocknote/react';
import { EntryRef } from './EntryRef';

/** Inline reference to a logbook entry: the blue chip from the mocks. The label is kept so exports work without the entry. */
export const entryRefSpec = createReactInlineContentSpec(
  { type: 'entryRef' as const, propSchema: { entryId: { default: '' }, label: { default: '' } }, content: 'none' },
  { render: (props) => <EntryRef entryId={props.inlineContent.props.entryId} label={props.inlineContent.props.label} /> },
);
