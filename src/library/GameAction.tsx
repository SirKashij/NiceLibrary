import { useRef, useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import type { Game, Operations } from '../operations';

/** Why the last main action did not do what it should, for the Game titled `title`. */
export type ActionProblem = { message: MessageKey; title: string };

/**
 * The main action on a Game: Launch for an Installed Game, Install for a
 * Steam Game that is owned but not Installed. Keeps why the latest of them
 * did not work, if it did not; an earlier one that answers later does not count.
 */
export function useGameAction(operations: Operations) {
  const [problem, setProblem] = useState<ActionProblem>();
  const latest = useRef(0);
  const run = (
    game: Game,
    action: Promise<{ outcome: string }>,
    messages: { steamNotFound: MessageKey; failed: MessageKey },
  ) => {
    const id = ++latest.current;
    const report = (message?: MessageKey) => {
      if (id === latest.current) setProblem(message && { message, title: game.title });
    };
    action.then(
      ({ outcome }) => report(outcome === 'steamNotFound' ? messages.steamNotFound : undefined),
      () => report(messages.failed),
    );
  };
  return {
    problem,
    clear: () => setProblem(undefined),
    play: (game: Game) =>
      run(game, operations.launch(game), { steamNotFound: 'launch.steamNotFound', failed: 'launch.failed' }),
    install: (game: Game) =>
      run(game, operations.install(game), { steamNotFound: 'install.steamNotFound', failed: 'install.failed' }),
  };
}

/** Whether `game` can be installed from the app: a Steam Game that is not Installed. */
export function installable(game: Game) {
  return game.source === 'steam' && !game.installed;
}

/**
 * The button for a Game's main action: Play with a play icon for an
 * Installed Game, Install with a download icon for a Steam Game that is not
 * Installed, nothing otherwise.
 */
export function GameActionButton({
  game,
  className,
  onPlay,
  onInstall,
}: {
  game: Game;
  className: string;
  onPlay: (game: Game) => void;
  onInstall: (game: Game) => void;
}) {
  const t = useT();
  if (game.installed) {
    return (
      <button
        type="button"
        className={className}
        aria-label={t('library.playGame', { title: game.title })}
        onClick={() => onPlay(game)}
      >
        <ActionIcon path={PLAY_ICON} />
        {t('library.play')}
      </button>
    );
  }
  if (installable(game)) {
    return (
      <button
        type="button"
        className={className}
        aria-label={t('library.installGame', { title: game.title })}
        onClick={() => onInstall(game)}
      >
        <ActionIcon path={INSTALL_ICON} />
        {t('library.install')}
      </button>
    );
  }
  return null;
}

const PLAY_ICON = 'M7 4.5v15l12-7.5z';
const INSTALL_ICON = 'M12 4v11M7 10.5l5 5 5-5M5 20h14';

function ActionIcon({ path }: { path: string }) {
  return (
    <svg className="icon action-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d={path} />
    </svg>
  );
}
