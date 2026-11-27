import { useEffect, useId, useState } from 'react';
import { useFormat, useT } from '../i18n';
import type { Game, Operations, Session } from '../operations';

/** How often the list is read again while one of its Sessions runs, so its length keeps growing. */
const RUNNING_REREAD_MS = 60_000;

type Translate = ReturnType<typeof useT>;

/**
 * The Sessions of a Game on its detail page, newest first, each with the day
 * and time it began and its length in hours and minutes; a Session whose
 * process still runs is marked as running. Reads them again whenever
 * `changes` changes (each Sync and each Session that begins or ends), and
 * every minute while one runs.
 */
export function SessionList({ operations, game, changes }: { operations: Operations; game: Game; changes: number }) {
  const t = useT();
  const format = useFormat();
  const headingId = useId();
  const [sessions, setSessions] = useState<Session[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [ticks, setTicks] = useState(0);
  const running = sessions?.some((session) => session.running) ?? false;

  useEffect(() => {
    let active = true;
    operations.sessions(game.id).then(
      (answer) => {
        if (!active) return;
        setSessions(answer);
        setFailed(false);
      },
      () => active && setFailed(true),
    );
    return () => {
      active = false;
    };
  }, [operations, game.id, changes, ticks]);

  useEffect(() => {
    if (!running) return;
    const timer = setInterval(() => setTicks((n) => n + 1), RUNNING_REREAD_MS);
    return () => clearInterval(timer);
  }, [running]);

  return (
    <section className="game-sessions" aria-labelledby={headingId}>
      <h2 id={headingId}>{t('sessions.title')}</h2>
      {failed ? (
        <p className="note" role="alert">
          {t('sessions.loadFailed', { title: game.title })}
        </p>
      ) : sessions === null ? null : sessions.length === 0 ? (
        <p className="muted">{t('sessions.none', { title: game.title })}</p>
      ) : (
        <ol className="session-list" aria-label={t('sessions.of', { title: game.title })}>
          {sessions.map((session) => (
            <li key={session.startedAt} data-running={session.running}>
              <time dateTime={new Date(session.startedAt * 1000).toISOString()}>{format.dateTime(session.startedAt)}</time>
              <span className="session-length">{sessionLength(t, format.count, session.seconds)}</span>
              {session.running && <span className="session-running">{t('sessions.running')}</span>}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

/** A Session's length in hours and minutes, or minutes alone below an hour. */
function sessionLength(t: Translate, count: (n: number) => string, seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  if (minutes < 1) return t('sessions.underAMinute');
  if (minutes < 60) return t('format.minutes', { n: count(minutes) });
  return t('sessions.hoursMinutes', { hours: count(Math.floor(minutes / 60)), minutes: count(minutes % 60) });
}
