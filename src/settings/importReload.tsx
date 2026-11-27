// After an import, everything the interface shows may have changed: the Library, the Collections, the
// language, the theme. Rather than telling each part, the whole interface starts over, as after a restart,
// on the settings page with the notice of the import.
import { createContext, Fragment, useContext, useState, type ReactNode } from 'react';
import type { DataImport } from '../operations';

/** An import that replaced the data. */
export type Imported = Extract<DataImport, { outcome: 'imported' }>;

interface ImportReload {
  /** The import the interface started over after; none before the first. */
  imported?: Imported;
  /** Starts the interface over after `imported`. */
  reload(imported: Imported): void;
}

const ImportReloadContext = createContext<ImportReload>({ reload: () => {} });

/** Starts `children` over, with fresh state, each time an import replaced the data. */
export function ImportReloadProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<{ round: number; imported?: Imported }>({ round: 0 });
  const reload = (imported: Imported) => setState((before) => ({ round: before.round + 1, imported }));
  return (
    <ImportReloadContext.Provider value={{ imported: state.imported, reload }}>
      <Fragment key={state.round}>{children}</Fragment>
    </ImportReloadContext.Provider>
  );
}

export function useImportReload(): ImportReload {
  return useContext(ImportReloadContext);
}
