import { useEffect, useId, useState } from 'react';
import { useFormat, useT } from '../i18n';
import { STATUS_LABELS, STATUSES } from '../library/Status';
import type { Game, Operations, Statistics } from '../operations';
import { MostPlayed } from './MostPlayed';
import { WeeklyPlaytime } from './WeeklyPlaytime';

type Answer = { state: 'loading' } | { state: 'ready'; statistics: Statistics } | { state: 'failed' };

/**
 * The Statistics page: the totals over the Library, the Playtime per week,
 * its Games by Status and the most played Games, as the core works them out,
 * without Hidden Games. Asks the core when it opens and again after each
 * change of the Library, such as a Sync.
 */
export function StatisticsPage({
  operations,
  syncs,
  onOpen,
}: {
  operations: Operations;
  /** Changes with each successful Sync or Session, which may change the figures. */
  syncs: number;
  /** Opens the detail page of a most played Game. */
  onOpen: (game: Game) => void;
}) {
  const t = useT();
  const headingId = useId();
  const [answer, setAnswer] = useState<Answer>({ state: 'loading' });

  useEffect(() => {
    let active = true;
    // Until the core answers again, the figures answered last stay.
    operations.statistics().then(
      (statistics) => active && setAnswer({ state: 'ready', statistics }),
      () => active && setAnswer({ state: 'failed' }),
    );
    return () => {
      active = false;
    };
  }, [operations, syncs]);

  return (
    <section className="page statistics" aria-labelledby={headingId}>
      <h1 id={headingId}>{t('nav.statistics')}</h1>
      {answer.state === 'failed' && (
        <p className="note" role="alert">
          {t('statistics.loadFailed')}
        </p>
      )}
      {answer.state === 'ready' &&
        (answer.statistics.games === 0 ? (
          <p className="note">{t('statistics.empty')}</p>
        ) : (
          <Figures statistics={answer.statistics} onOpen={onOpen} />
        ))}
    </section>
  );
}

/** The totals, the Playtime per week, how many Games have each Status, then the most played Games. */
function Figures({ statistics, onOpen }: { statistics: Statistics; onOpen: (game: Game) => void }) {
  const t = useT();
  const format = useFormat();
  const byStatusId = useId();
  const achievements =
    statistics.achievementShare === null
      ? t('statistics.noAchievements')
      : t('statistics.achievementsOf', {
          unlocked: format.count(statistics.achievementsUnlocked),
          total: format.count(statistics.achievementsTotal),
          share: format.wholePercent(statistics.achievementShare),
        });

  return (
    <>
      <dl className="statistics-figures" aria-label={t('statistics.totals')}>
        <div>
          <dt>{t('statistics.playtime')}</dt>
          <dd>{format.playtime(statistics.playtimeMinutes)}</dd>
        </div>
        <div>
          <dt>{t('statistics.achievements')}</dt>
          <dd>{achievements}</dd>
        </div>
        <div>
          <dt>{t('statistics.completeGames')}</dt>
          <dd>{format.count(statistics.completeGames)}</dd>
        </div>
      </dl>
      <WeeklyPlaytime weeks={statistics.weeks} />
      <section className="statistics-section" aria-labelledby={byStatusId}>
        <h2 id={byStatusId}>{t('statistics.byStatus')}</h2>
        <dl className="statistics-by-status">
          {STATUSES.map((status) => (
            <div key={status} data-status={status}>
              <dt>{t(STATUS_LABELS[status])}</dt>
              <dd>{format.count(statistics.gamesByStatus[status])}</dd>
            </div>
          ))}
        </dl>
      </section>
      <MostPlayed games={statistics.mostPlayed} onOpen={onOpen} />
    </>
  );
}
