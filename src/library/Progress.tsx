import { useFormat, useT } from '../i18n';
import type { Game } from '../operations';

/** The highest Progress, in percent: the Game is complete (see `COMPLETE` in crates/core). */
const COMPLETE = 100;

/**
 * A Game's Progress under its Cover, a complete Game in its own colour.
 * Nothing for a Game without Progress, never 0 % in its place.
 */
export function ProgressBadge({ progress }: Pick<Game, 'progress'>) {
  const t = useT();
  const format = useFormat();
  if (progress === null) return null;
  return (
    <span className="game-progress" data-complete={progress >= COMPLETE}>
      {t('library.progress', { percent: format.percent(progress) })}
    </span>
  );
}

/** A Game's Progress as a value, on its detail page, a complete Game in the same colour as on its cover card. */
export function ProgressValue({ progress }: { progress: number }) {
  const format = useFormat();
  return (
    <span className="progress-value" data-complete={progress >= COMPLETE}>
      {format.percent(progress)}
    </span>
  );
}
