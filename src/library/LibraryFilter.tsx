import { useEffect, useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import type { Game, LibraryFilter, Operations, Source } from '../operations';
import { STATUS_LABELS, STATUSES } from './Status';

/** The filter that narrows nothing: the whole Library. */
export const NO_FILTER: LibraryFilter = { search: '', source: null, installedOnly: false, status: null };

/** Whether `filter` narrows the Library at all. */
export function isFiltering(filter: LibraryFilter): boolean {
  return filter.search !== '' || filter.source !== null || filter.installedOnly || filter.status !== null;
}

/** What the Library shows for a filter: the Games, or that the core could not filter. */
export type FilteredLibrary = { state: 'ready'; games: Game[] | undefined } | { state: 'failed' };

/**
 * The Games to show for `filter`. Without a filter that is `library` itself;
 * otherwise the core answers (see `filterLibrary` in src/operations.ts),
 * asked again whenever the filter or the Library changes. Until the answer
 * for a new filter arrives, the answer before stays.
 */
export function useFilteredLibrary(
  operations: Operations,
  filter: LibraryFilter,
  library: Game[] | undefined,
): FilteredLibrary {
  const filtering = isFiltering(filter);
  const [answer, setAnswer] = useState<FilteredLibrary>({ state: 'ready', games: undefined });

  useEffect(() => {
    // Once the filters are cleared, the next filter starts again from the whole Library.
    if (!filtering) setAnswer({ state: 'ready', games: undefined });
    if (!filtering || library === undefined) return;
    // Only the answer to the latest filter counts; an earlier one may answer later.
    let latest = true;
    operations.filterLibrary(filter).then(
      (games) => latest && setAnswer({ state: 'ready', games }),
      () => latest && setAnswer({ state: 'failed' }),
    );
    return () => {
      latest = false;
    };
  }, [operations, filter, filtering, library]);

  if (!filtering) return { state: 'ready', games: library };
  if (answer.state === 'ready' && answer.games === undefined) return { state: 'ready', games: library };
  return answer;
}

const SOURCES: { source: Source | null; label: MessageKey }[] = [
  { source: null, label: 'filter.allSources' },
  { source: 'steam', label: 'filter.steam' },
  { source: 'local', label: 'filter.local' },
];

/** The toolbar above the Library: search, Source, Status and Installed only. */
export function LibraryToolbar({
  filter,
  onChange,
}: {
  filter: LibraryFilter;
  onChange: (filter: LibraryFilter) => void;
}) {
  const t = useT();
  return (
    <div className="toolbar">
      <input
        type="search"
        className="toolbar-search"
        aria-label={t('filter.search')}
        placeholder={t('filter.searchPlaceholder')}
        value={filter.search}
        onChange={(event) => onChange({ ...filter, search: event.target.value })}
      />
      <div className="segmented" role="group" aria-label={t('filter.source')}>
        {SOURCES.map(({ source, label }) => (
          <button
            key={label}
            type="button"
            aria-pressed={filter.source === source}
            onClick={() => onChange({ ...filter, source })}
          >
            {t(label)}
          </button>
        ))}
      </div>
      <select
        className="toolbar-select"
        aria-label={t('filter.status')}
        value={filter.status ?? ''}
        onChange={(event) => onChange({ ...filter, status: STATUSES.find((status) => status === event.target.value) ?? null })}
      >
        <option value="">{t('filter.anyStatus')}</option>
        {STATUSES.map((status) => (
          <option key={status} value={status}>
            {t(STATUS_LABELS[status])}
          </option>
        ))}
      </select>
      <label className="toolbar-toggle">
        <input
          type="checkbox"
          checked={filter.installedOnly}
          onChange={(event) => onChange({ ...filter, installedOnly: event.target.checked })}
        />
        {t('filter.installedOnly')}
      </label>
    </div>
  );
}

/** Shown when the filters and the search match no Game; its action clears them all. */
export function NoMatch({ onReset }: { onReset: () => void }) {
  const t = useT();
  return (
    <div className="empty-state">
      <p className="note">{t('filter.noMatch')}</p>
      <button type="button" className="button" onClick={onReset}>
        {t('filter.reset')}
      </button>
    </div>
  );
}
