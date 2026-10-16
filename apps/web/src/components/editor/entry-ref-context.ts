import { createContext, useContext } from 'react';

/** Lets an `entryRef` chip open the entry drawer without the editor knowing about features. */
export const EntryRefContext = createContext<(entryId: number) => void>(() => {});
export const useOpenEntryRef = () => useContext(EntryRefContext);
