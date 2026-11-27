import { useId } from 'react';
import { useFormat, useT } from '../i18n';
import type { Game } from '../operations';

/**
 * The most played Games, most played first, each with its rank, title,
 * Playtime and a bar as long as its share of the first one's. Choosing a
 * Game opens its detail page.
 */
export function MostPlayed({ games, onOpen }: { games: Game[]; onOpen: (game: Game) => void }) {
  const t = useT();
  const format = useFormat();
  const headingId = useId();
  const most = Math.max(0, ...games.map((game) => game.playtimeMinutes ?? 0));

  return (
    <section className="statistics-section" aria-labelledby={headingId}>
      <h2 id={headingId}>{t('statistics.mostPlayed')}</h2>
      {games.length === 0 ? (
        <p className="note">{t('statistics.noneMostPlayed')}</p>
      ) : (
        <ol className="most-played" aria-labelledby={headingId}>
          {games.map((game, n) => {
            const minutes = game.playtimeMinutes ?? 0;
            return (
              <li key={game.id}>
                <button type="button" onClick={() => onOpen(game)}>
                  <span className="most-played-rank">{format.count(n + 1)}</span>{' '}
                  <span className="most-played-title">{game.title}</span>{' '}
                  <span className="most-played-share" aria-hidden="true">
                    <span style={{ width: `${most === 0 ? 0 : (minutes / most) * 100}%` }} />
                  </span>
                  <span className="most-played-playtime">{format.playtime(minutes)}</span>
                </button>
              </li>
            );
          })}
        </ol>
      )}
    </section>
  );
}
