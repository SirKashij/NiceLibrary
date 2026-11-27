import { useEffect, useId, useRef, useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import type { LibrarySort, Operations } from '../operations';

/** The order `library` answers in (see `Core::library` in crates/core): by title. */
export const LIBRARY_ORDER: LibrarySort = 'name';

/** The order of the Library until the user chooses one, or when the chosen one cannot be read. */
const DEFAULT_SORT: LibrarySort = 'recentlyPlayed';

const SORTS: { sort: LibrarySort; label: MessageKey }[] = [
  { sort: 'recentlyPlayed', label: 'sort.recentlyPlayed' },
  { sort: 'name', label: 'sort.name' },
  { sort: 'playtime', label: 'sort.playtime' },
];

/** The order the Library is shown in, and how the user changes it. */
export interface LibrarySortChoice {
  /** `undefined` until the order chosen before is known. */
  sort: LibrarySort | undefined;
  /** Shows the Library in `sort` at once and keeps it for the next start. */
  choose: (sort: LibrarySort) => void;
  /** Whether the order chosen last could not be kept for the next start. */
  notKept: boolean;
}

/**
 * The order of the Library: the one the user chose before (see `librarySort`
 * in src/operations.ts), until they choose another.
 */
export function useLibrarySort(operations: Operations): LibrarySortChoice {
  const [sort, setSort] = useState<LibrarySort>();
  const [notKept, setNotKept] = useState(false);
  // Only the answer to the latest choice counts; an earlier one may answer later.
  const latestChoice = useRef(0);

  useEffect(() => {
    let active = true;
    // A choice made while the stored order is read wins over it.
    operations.librarySort().then(
      (stored) => active && setSort((current) => current ?? stored),
      () => active && setSort((current) => current ?? DEFAULT_SORT),
    );
    return () => {
      active = false;
    };
  }, [operations]);

  const choose = (next: LibrarySort) => {
    const id = ++latestChoice.current;
    setSort(next);
    setNotKept(false);
    operations.chooseLibrarySort(next).then(
      () => {},
      () => id === latestChoice.current && setNotKept(true),
    );
  };

  return { sort, choose, notKept };
}

/** The sort order in the toolbar above the Library. */
export function SortChoice({ sort, choose }: Pick<LibrarySortChoice, 'sort' | 'choose'>) {
  const t = useT();
  const id = useId();
  return (
    <div className="toolbar-sort">
      <label htmlFor={id}>{t('sort.label')}</label>
      <select
        id={id}
        value={sort ?? DEFAULT_SORT}
        disabled={sort === undefined}
        onChange={(event) => choose(event.target.value as LibrarySort)}
      >
        {SORTS.map(({ sort, label }) => (
          <option key={sort} value={sort}>
            {t(label)}
          </option>
        ))}
      </select>
    </div>
  );
}
