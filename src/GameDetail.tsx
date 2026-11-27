import { useEffect, useId, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { Achievements } from './Achievements';
import { useCoverTint } from './coverTint';
import { useFormat, useT, type MessageKey } from './i18n';
import { FavoriteToggle } from './Favorites';
import { LaunchOptionsForm } from './LaunchOptions';
import { GameActionButton } from './library/GameAction';
import { HideGameButton } from './library/HiddenGames';
import { StatusChoice } from './library/Status';
import { LocalGameCoverChoice } from './LocalGameCover';
import { CoverMatchChoice } from './CoverMatch';
import type { Game, Operations, Source, Status } from './operations';

const SOURCES: Record<Source, MessageKey> = {
  steam: 'game.sourceSteam',
  local: 'game.sourceLocal',
};

/**
 * The detail page of one Game: its large Cover, title, Source, whether it is
 * Installed, its Status to set, Playtime, Last Played, its Collections and
 * the main action, its Achievements, marking it as a Favorite, and the way
 * back to the Library. Shows the Game as opened at once and as the core
 * answers it then, again after each change. Takes its Tint from the Game's
 * Cover, or stays neutral (see src/coverTint.ts).
 */
export function GameDetail({
  operations,
  game: opened,
  syncs,
  cover,
  collections,
  notice,
  onPlay,
  onInstall,
  onStatusSet,
  onFavorite,
  onBack,
  onHidden,
  onCoverChanged,
  actions,
}: {
  operations: Operations;
  /** The Game as the Library showed it when it was opened. */
  game: Game;
  /** Changes with each successful Sync, edit and Favorite mark taken, which may change the Game. */
  syncs: number;
  /** Further actions on the Game as the core answered it, such as editing a Local Game. */
  actions?: (game: Game) => ReactNode;
  cover: (game: Game) => ReactNode;
  /** Which Collections the Game is in, shown while it is in the Library. */
  collections?: ReactNode;
  /** Why the last Launch or Install did not work, if it did not. */
  notice?: ReactNode;
  onPlay: (game: Game) => void;
  /** Starts the installation of a Steam Game that is not Installed. */
  onInstall: (game: Game) => void;
  /** Hears that the Game's Status was set, which changes the Library. */
  onStatusSet: (game: Game) => void;
  /** Marks the Game as a Favorite, or unmarks it. */
  onFavorite: (game: Game) => void;
  onBack: () => void;
  /** Hears the Game once the user hid it. */
  onHidden: (game: Game) => void;
  /** Hears that the user changed the Game's Cover, so it is loaded again. */
  onCoverChanged: (game: Game) => void;
}) {
  const t = useT();
  const format = useFormat();
  const headingId = useId();
  // `null` once the core answers that the Game is no longer in the Library.
  const [game, setGame] = useState<Game | null>(opened);
  const [statusFailed, setStatusFailed] = useState(false);
  // Only the answer to the latest choice counts; an earlier one may answer later.
  const latestStatus = useRef(0);
  const setStatus = (status: Status) => {
    const id = ++latestStatus.current;
    setStatusFailed(false);
    operations.setStatus(opened.id, status).then(
      (answer) => {
        if (id !== latestStatus.current) return;
        setGame(answer);
        onStatusSet(answer);
      },
      () => id === latestStatus.current && setStatusFailed(true),
    );
  };

  useEffect(() => {
    let active = true;
    // A Game that cannot be read now stays as shown; the next answer replaces it.
    operations.game(opened.id).then(
      (answer) => active && setGame(answer),
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations, opened.id, syncs]);

  // A Local Game's Cover changes when the user chooses an image, a Cover
  // Match or goes back to the icon; its Tint follows.
  const [coverChanges, setCoverChanges] = useState(0);
  const coverChanged = (changed: Game) => {
    setCoverChanges((n) => n + 1);
    onCoverChanged(changed);
  };
  const tint = useCoverTint(operations, opened.id, coverChanges);
  // The Tint replaces the theme's background and accent for everything on the page.
  const tinted = tint
    ? ({ '--color-bg': tint.background, '--color-accent': tint.accent, '--color-accent-ink': tint.accentInk } as CSSProperties)
    : undefined;

  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => heading.current?.focus(), []);

  const minutes = game?.playtimeMinutes ?? null;
  const playtime = minutes === null ? t('game.notKnown') : minutes === 0 ? t('library.neverPlayed') : format.playtime(minutes);
  const lastPlayed =
    game?.lastPlayed != null ? format.date(game.lastPlayed) : minutes === null ? t('game.notKnown') : t('game.never');

  return (
    <section
      className="page game-detail"
      aria-labelledby={headingId}
      data-installed={game?.installed ?? false}
      data-tint={tint ? 'cover' : 'neutral'}
      style={tinted}
    >
      <button type="button" className="back" onClick={onBack}>
        {t('game.back')}
      </button>
      {notice}
      <div className="game-detail-body">
        <div className="game-detail-cover">
          {cover(game ?? opened)}
          {game?.source === 'local' && (
            <>
              <LocalGameCoverChoice
                operations={operations}
                game={game}
                onCoverChanged={coverChanged}
              />
              <CoverMatchChoice operations={operations} game={game} onCoverChanged={coverChanged} />
            </>
          )}
          {game && <FavoriteToggle game={game} onToggle={onFavorite} />}
        </div>
        <div className="game-detail-info">
          <h1 id={headingId} ref={heading} tabIndex={-1}>
            {game?.title ?? opened.title}
          </h1>
          {game === null ? (
            <p className="note">{t('game.gone', { title: opened.title })}</p>
          ) : (
            <>
              <GameActionButton game={game} className="button play-main" onPlay={onPlay} onInstall={onInstall} />
              <dl className="game-facts" aria-label={t('game.facts', { title: game.title })}>
                <div>
                  <dt>{t('game.source')}</dt>
                  <dd>{t(SOURCES[game.source])}</dd>
                </div>
                <div>
                  <dt>{t('game.installed')}</dt>
                  <dd>{t(game.installed ? 'game.yes' : 'game.no')}</dd>
                </div>
                <div>
                  <dt>{t('game.status')}</dt>
                  <dd>
                    <StatusChoice title={game.title} status={game.status} onChoose={setStatus} />
                  </dd>
                </div>
                <div>
                  <dt>{t('game.playtime')}</dt>
                  <dd>{playtime}</dd>
                </div>
                <div>
                  <dt>{t('game.lastPlayed')}</dt>
                  <dd>{lastPlayed}</dd>
                </div>
              </dl>
              {statusFailed && (
                <p className="note" role="alert">
                  {t('status.setFailed', { title: game.title })}
                </p>
              )}
              {collections}
              <HideGameButton operations={operations} game={game} onHidden={onHidden} />
              {game.source === 'local' && <LaunchOptionsForm operations={operations} game={game} />}
              {actions?.(game)}
            </>
          )}
        </div>
      </div>
      {game !== null && <Achievements operations={operations} game={game} syncs={syncs} />}
    </section>
  );
}
