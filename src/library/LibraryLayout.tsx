import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useFormat, useT, type MessageKey } from '../i18n';
import { FavoriteToggle } from '../Favorites';
import type { Game, LibraryLayout, Operations, Source } from '../operations';
import { STATUS_LABELS } from './Status';

/** The Library Layout the user sees and how they switch it. */
export interface LibraryLayoutChoice {
  /** The Library Layout to show; `undefined` until the stored one answers. */
  layout: LibraryLayout | undefined;
  /** Shows `layout` at once and keeps it; goes back to the one before if it cannot be kept. */
  choose(layout: LibraryLayout): void;
  /** Whether the latest choice could not be kept. */
  saveFailed: boolean;
}

/**
 * The stored Library Layout (see `libraryLayout` in src/operations.ts): the
 * cover grid when it cannot be read, and from the user's first switch on
 * their choice, whatever the stored one answers later.
 */
export function useLibraryLayout(operations: Operations): LibraryLayoutChoice {
  const [layout, setLayout] = useState<LibraryLayout>();
  const [saveFailed, setSaveFailed] = useState(false);
  const chosenHere = useRef(false);
  // Only the answer to the latest choice counts; an earlier one may answer later.
  const latestChoice = useRef(0);

  useEffect(() => {
    let active = true;
    const answer = (stored: LibraryLayout) => active && !chosenHere.current && setLayout(stored);
    operations.libraryLayout().then(answer, () => answer('coverGrid'));
    return () => {
      active = false;
    };
  }, [operations]);

  const choose = (next: LibraryLayout) => {
    const id = ++latestChoice.current;
    const before = layout ?? 'coverGrid';
    chosenHere.current = true;
    setLayout(next);
    setSaveFailed(false);
    operations.chooseLibraryLayout(next).then(
      () => {},
      () => {
        if (id !== latestChoice.current) return;
        setLayout(before);
        setSaveFailed(true);
      },
    );
  };

  return { layout, choose, saveFailed };
}

const LAYOUTS: { layout: LibraryLayout; label: MessageKey }[] = [
  { layout: 'coverGrid', label: 'layout.coverGrid' },
  { layout: 'list', label: 'layout.list' },
];

/** The switch between the cover grid and the list, in the Library's toolbar. */
export function LibraryLayoutSwitch({ choice }: { choice: LibraryLayoutChoice }) {
  const t = useT();
  return (
    <div className="segmented" role="group" aria-label={t('layout.choose')}>
      {LAYOUTS.map(({ layout, label }) => (
        <button
          key={layout}
          type="button"
          aria-pressed={(choice.layout ?? 'coverGrid') === layout}
          onClick={() => choice.choose(layout)}
        >
          {t(label)}
        </button>
      ))}
    </div>
  );
}

const SOURCES: Record<Source, MessageKey> = {
  steam: 'game.sourceSteam',
  local: 'game.sourceLocal',
};

/**
 * The Library as a list: one row per Game with its small Cover, title,
 * Status, Source, Playtime, Last Played, its Favorite star and main action,
 * in the order given, the same as the cover grid's. The whole row opens the
 * Game's detail page.
 */
export function GameList({
  games,
  cover,
  onPlay,
  onOpen,
  onFavorite,
}: {
  games: Game[];
  cover: (game: Game) => ReactNode;
  onPlay: (game: Game) => void;
  /** Opens the Game's detail page. */
  onOpen: (game: Game) => void;
  /** Marks the Game as a Favorite, or unmarks it. */
  onFavorite: (game: Game) => void;
}) {
  const t = useT();
  return (
    <div className="game-list">
      <div className="game-row game-list-head" aria-hidden="true">
        <span />
        <span>{t('layout.titleColumn')}</span>
        <span>{t('game.status')}</span>
        <span>{t('game.source')}</span>
        <span>{t('game.playtime')}</span>
        <span>{t('game.lastPlayed')}</span>
        <span />
      </div>
      <ul aria-label={t('library.games')}>
        {games.map((game) => (
          <li key={game.id} className="game-row" data-installed={game.installed}>
            <button
              type="button"
              className="open-row"
              aria-label={t('library.openGame', { title: game.title })}
              onClick={() => onOpen(game)}
            >
              {cover(game)}
            </button>
            <h2 className="game-title">{game.title}</h2>
            <Fact label="game.status" value={t(STATUS_LABELS[game.status])} />
            <Fact label="game.source" value={t(SOURCES[game.source])} />
            <Facts game={game} />
            <span className="game-row-action">
              <FavoriteToggle game={game} onToggle={onFavorite} />
              {game.installed ? (
                <button
                  type="button"
                  className="button play-row"
                  aria-label={t('library.playGame', { title: game.title })}
                  onClick={() => onPlay(game)}
                >
                  {t('library.play')}
                </button>
              ) : (
                <span className="muted">{t('library.notInstalled')}</span>
              )}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** A Game's Playtime and Last Played, as its detail page names them. */
function Facts({ game }: { game: Game }) {
  const t = useT();
  const format = useFormat();
  const { playtimeMinutes: minutes, lastPlayed } = game;
  const playtime = minutes === null ? t('game.notKnown') : minutes === 0 ? t('library.neverPlayed') : format.playtime(minutes);
  const last = lastPlayed !== null ? format.date(lastPlayed) : minutes === null ? t('game.notKnown') : t('game.never');
  return (
    <>
      <Fact label="game.playtime" value={playtime} />
      <Fact label="game.lastPlayed" value={last} />
    </>
  );
}

/** One column of a row; screen readers hear its name, which the list's head shows. */
function Fact({ label, value }: { label: MessageKey; value: string }) {
  const t = useT();
  return (
    <span className="game-row-fact">
      <span className="visually-hidden">{t(label)} </span>
      <span>{value}</span>
    </span>
  );
}
