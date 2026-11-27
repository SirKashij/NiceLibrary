import { useEffect, useId, useRef, useState } from 'react';
import { useT } from '../i18n';
import type { Game, Operations } from '../operations';

/**
 * Hides a Game, on its detail page. `onHidden` hears the Game once the core
 * hid it; when it could not, the button stays and says so.
 */
export function HideGameButton({
  operations,
  game,
  onHidden,
}: {
  operations: Operations;
  game: Game;
  onHidden: (game: Game) => void;
}) {
  const t = useT();
  const [hiding, setHiding] = useState(false);
  const [failed, setFailed] = useState(false);

  const hide = () => {
    setHiding(true);
    setFailed(false);
    operations.hideGame(game.id).then(
      () => onHidden(game),
      () => {
        setHiding(false);
        setFailed(true);
      },
    );
  };

  return (
    <div className="hide-game">
      <button
        type="button"
        className="button"
        aria-label={t('hidden.hideGame', { title: game.title })}
        disabled={hiding}
        onClick={hide}
      >
        {t('hidden.hide')}
      </button>
      {failed && (
        <p className="note" role="alert">
          {t('hidden.hideFailed', { title: game.title })}
        </p>
      )}
    </div>
  );
}

/**
 * The Hidden Games, each with the way to unhide it, and the way back to the
 * Library. `onUnhidden` hears each Game the core brought back.
 */
export function HiddenGamesPage({
  operations,
  onUnhidden,
  onBack,
}: {
  operations: Operations;
  onUnhidden: (game: Game) => void;
  onBack: () => void;
}) {
  const t = useT();
  const headingId = useId();
  const [games, setGames] = useState<Game[]>();
  const [loadFailed, setLoadFailed] = useState(false);
  // The Game that could not be unhidden last, if one could not.
  const [unhideFailed, setUnhideFailed] = useState<Game>();

  useEffect(() => {
    let active = true;
    operations.hiddenGames().then(
      (answer) => active && setGames(answer),
      () => active && setLoadFailed(true),
    );
    return () => {
      active = false;
    };
  }, [operations]);

  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => heading.current?.focus(), []);

  const unhide = (game: Game) => {
    setUnhideFailed(undefined);
    operations.unhideGame(game.id).then(
      () => {
        setGames((shown) => shown?.filter((other) => other.id !== game.id));
        onUnhidden(game);
      },
      () => setUnhideFailed(game),
    );
  };

  return (
    <section className="page hidden-games" aria-labelledby={headingId}>
      <button type="button" className="back" onClick={onBack}>
        {t('game.back')}
      </button>
      <h1 id={headingId} ref={heading} tabIndex={-1}>
        {t('hidden.title')}
      </h1>
      {loadFailed && (
        <p className="note" role="alert">
          {t('hidden.loadFailed')}
        </p>
      )}
      {unhideFailed && (
        <p className="note" role="alert">
          {t('hidden.unhideFailed', { title: unhideFailed.title })}
        </p>
      )}
      {games?.length === 0 && <p className="note">{t('hidden.none')}</p>}
      {games && games.length > 0 && (
        <ul className="hidden-games-list" aria-label={t('hidden.title')}>
          {games.map((game) => (
            <li key={game.id}>
              <span>{game.title}</span>
              <button
                type="button"
                className="button"
                aria-label={t('hidden.unhideGame', { title: game.title })}
                onClick={() => unhide(game)}
              >
                {t('hidden.unhide')}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
