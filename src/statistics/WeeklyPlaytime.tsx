import { useId } from 'react';
import { useFormat, useT } from '../i18n';
import type { PlayedWeek } from '../operations';

/**
 * The Playtime of each of the last eight weeks as a column chart, oldest
 * week first: each bar labelled with its Playtime and its week, as high as
 * its share of the longest week. Pointing at a week names its days. The
 * chart says that it counts only the time the app recorded.
 */
export function WeeklyPlaytime({ weeks }: { weeks: PlayedWeek[] }) {
  const t = useT();
  const format = useFormat();
  const headingId = useId();
  const longest = Math.max(0, ...weeks.map((week) => week.minutes));

  return (
    <section className="statistics-section" aria-labelledby={headingId}>
      <h2 id={headingId}>{t('statistics.weeks')}</h2>
      <p className="muted">{t('statistics.weeksNote')}</p>
      <ol className="week-chart" aria-labelledby={headingId}>
        {weeks.map((week, n) => {
          const ago = weeks.length - 1 - n;
          const playtime = format.playtime(week.minutes);
          return (
            <li
              key={week.startsAt}
              aria-current={ago === 0 ? 'date' : undefined}
              title={t('statistics.weekRange', {
                from: format.date(week.startsAt),
                to: format.date(week.endsAt),
                playtime,
              })}
            >
              <span className="week-value">{playtime}</span>{' '}
              <span className="week-track" aria-hidden="true">
                <span
                  className="week-bar"
                  style={{ height: `${longest === 0 ? 0 : (week.minutes / longest) * 100}%` }}
                />
              </span>{' '}
              <span className="week-label">
                {ago === 0
                  ? t('statistics.thisWeek')
                  : ago === 1
                    ? t('statistics.weekAgo')
                    : t('statistics.weeksAgo', { n: String(ago) })}
              </span>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
