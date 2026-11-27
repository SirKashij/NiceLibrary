import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { useFormat, useT, type MessageKey } from './i18n';
import { SessionList } from './library/SessionList';
import { StatusChoice } from './library/Status';
import type { Game, Operations, Source, Status } from './operations';

const SOURCES: Record<Source, MessageKey> = {
  steam: 'game.sourceSteam',
  local: 'game.sourceLocal',
};

/**
 * The detail page of one Game: its large Cover, title, Source, whether it is
 * Installed, its Status to set, Playtime, Last Played, its Sessions and the main action, and the way back to
 * the Library. Shows the Game as opened at once and as the core answers it
 * then, again after each Sync.
 */
export function GameDetail({
  operations,
  game: opened,
  syncs,
  cover,
  notice,
  onPlay,
  onStatusSet,
  onBack,
}: {
  operations: Operations;
  /** The Game as the Library showed it when it was opened. */
  game: Game;
  /** Changes with each successful Sync, which may change the Game. */
  syncs: number;
  cover: (game: Game) => ReactNode;
  /** Why the last Launch did not start, if it did not. */
  notice?: ReactNode;
  onPlay: (game: Game) => void;
  /** Hears that the Game's Status was set, which changes the Library. */
  onStatusSet: (game: Game) => void;
  onBack: () => void;
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

  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => heading.current?.focus(), []);

  const minutes = game?.playtimeMinutes ?? null;
  const playtime = minutes === null ? t('game.notKnown') : minutes === 0 ? t('library.neverPlayed') : format.playtime(minutes);
  const lastPlayed =
    game?.lastPlayed != null ? format.date(game.lastPlayed) : minutes === null ? t('game.notKnown') : t('game.never');

  return (
    <section className="page game-detail" aria-labelledby={headingId} data-installed={game?.installed ?? false}>
      <button type="button" className="back" onClick={onBack}>
        {t('game.back')}
      </button>
      {notice}
      <div className="game-detail-body">
        <div className="game-detail-cover">{cover(game ?? opened)}</div>
        <div className="game-detail-info">
          <h1 id={headingId} ref={heading} tabIndex={-1}>
            {opened.title}
          </h1>
          {game === null ? (
            <p className="note">{t('game.gone', { title: opened.title })}</p>
          ) : (
            <>
              {game.installed && (
                <button
                  type="button"
                  className="button play-main"
                  aria-label={t('library.playGame', { title: game.title })}
                  onClick={() => onPlay(game)}
                >
                  {t('library.play')}
                </button>
              )}
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
              <SessionList operations={operations} game={game} changes={syncs} />
            </>
          )}
        </div>
      </div>
    </section>
  );
}
