import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useFormat, useT } from './i18n';
import type { Game, Operations } from './operations';

/** A five-pointed star; the outline of a Favorite's marker, filled once marked. */
const STAR = 'M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.8-5.2-2.8-5.2 2.8 1-5.8-4.3-4.1 5.9-.8z';

/**
 * The star on a Game's Cover: marks the Game as a Favorite, or unmarks it.
 * Pressed, it is the marker every Favorite shows on its Cover.
 */
export function FavoriteToggle({ game, onToggle }: { game: Game; onToggle: (game: Game) => void }) {
  const t = useT();
  return (
    <button
      type="button"
      className="favorite-toggle"
      aria-pressed={game.favorite}
      aria-label={t('favorites.toggle', { title: game.title })}
      onClick={() => onToggle(game)}
    >
      <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path d={STAR} />
      </svg>
    </button>
  );
}

/**
 * Marking and unmarking Favorites: `toggle` asks the core to flip a Game's
 * mark and `onMarked` hears each mark the core took, after which the
 * Library is read again. `notice` says when a mark could not be changed.
 */
export function useFavoriteMarking(
  operations: Operations,
  onMarked: () => void,
): { toggle: (game: Game) => void; notice: ReactNode } {
  const t = useT();
  // The title of the Game whose mark could not be changed last, if one could not.
  const [failed, setFailed] = useState<string>();
  const toggle = (game: Game) => {
    setFailed(undefined);
    operations.markFavorite(game.id, !game.favorite).then(onMarked, () => setFailed(game.title));
  };
  const notice = failed !== undefined && (
    <p className="note" role="alert">
      {t('favorites.failed', { title: failed })}
    </p>
  );
  return { toggle, notice };
}

/**
 * The Favorites entry in the sidebar, with how many Games of the Library are
 * Favorites; it narrows the Library to them. The count is asked for again
 * whenever `changes` changes.
 */
export function FavoritesEntry({
  operations,
  changes,
  active,
  onOpen,
}: {
  operations: Operations;
  /** Changes with each mark taken and each Sync, which may change the count. */
  changes: number;
  /** Whether the Library shows only the Favorites now. */
  active: boolean;
  onOpen: () => void;
}) {
  const t = useT();
  const format = useFormat();
  const [count, setCount] = useState<number>();
  // Only the answer to the latest question counts.
  const latest = useRef(0);

  useEffect(() => {
    const id = ++latest.current;
    // A count that cannot be read now stays as shown; the next answer replaces it.
    operations.favoriteCount().then(
      (answer) => id === latest.current && setCount(answer),
      () => {},
    );
  }, [operations, changes]);

  const label =
    count === undefined
      ? t('favorites.title')
      : count === 1
        ? t('favorites.entryOne')
        : t('favorites.entry', { count: format.count(count) });
  return (
    <button
      type="button"
      className="favorites-entry"
      aria-current={active ? 'page' : undefined}
      aria-label={label}
      onClick={onOpen}
    >
      <span>{t('favorites.title')}</span>
      {count !== undefined && <span className="favorites-count">{count}</span>}
    </button>
  );
}

/** Shown in place of the Library while it shows only Favorites and there is none. */
export function NoFavorites() {
  const t = useT();
  return <p className="note">{t('favorites.none')}</p>;
}
