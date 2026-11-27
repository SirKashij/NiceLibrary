import { useEffect, useRef, useState } from 'react';
import { useFormat, useT, type MessageKey } from '../i18n';
import type { Operations, SteamAppList, SteamAppListFetch } from '../operations';

/** A message under the setting: `status` for what went well, `alert` for what did not. */
type Notice = { role: 'status' | 'alert'; message: MessageKey };

/** What a fetch that did not bring the whole list answers, for the user. */
const FETCH_PROBLEMS: Record<Exclude<SteamAppListFetch['outcome'], 'fetched'>, MessageKey> = {
  noKey: 'appList.noKey',
  keyRejected: 'appList.keyRejected',
  steamUnreachable: 'appList.steamUnreachable',
};

/**
 * The Steam App List, which Cover Matches are looked up in: how much of it
 * is stored and since when, and fetching it now. Fetches the core runs on
 * its own show here as they happen.
 */
export function SteamAppListSettings({ operations }: { operations: Operations }) {
  const t = useT();
  const format = useFormat();
  // `undefined` while the stored state is being read, `null` when it could not be.
  const [list, setList] = useState<SteamAppList | null>();
  const [fetching, setFetching] = useState(false);
  const [notice, setNotice] = useState<Notice>();
  const fetchingRef = useRef(false);

  const read = () =>
    operations.steamAppList().then(
      (value) => setList(value),
      () => setList(null),
    );

  const begin = () => {
    fetchingRef.current = true;
    setFetching(true);
    setNotice(undefined);
  };
  /** Shows how a fetch ended, asked for here or started by the core on its own. */
  const settle = (answer: SteamAppListFetch | 'failed') => {
    fetchingRef.current = false;
    setFetching(false);
    if (answer === 'failed') {
      setNotice({ role: 'alert', message: 'appList.failed' });
    } else if (answer.outcome === 'fetched') {
      setNotice({ role: 'status', message: 'appList.upToDate' });
    } else {
      setNotice({ role: 'alert', message: FETCH_PROBLEMS[answer.outcome] });
    }
    // Pages may be stored whatever the outcome.
    void read();
  };
  // The latest render's handlers, for the listener that lives as long as the view.
  const handlers = useRef({ begin, settle });
  handlers.current = { begin, settle };

  useEffect(() => {
    void read();
    // `read` only names which operation to run and is the same for the life of the view.
  }, [operations]);

  useEffect(
    () =>
      operations.onAutomaticSteamAppListFetch((event) => {
        if (event.state === 'started') handlers.current.begin();
        else handlers.current.settle(event.state === 'failed' ? 'failed' : event);
      }),
    [operations],
  );

  const fetchNow = () => {
    if (fetchingRef.current) return;
    begin();
    operations.fetchSteamAppList().then(settle, () => settle('failed'));
  };

  return (
    <section className="settings-section" aria-labelledby="settings-app-list">
      <h2 id="settings-app-list">{t('settings.appList')}</h2>
      <p className="note">{t('appList.why')}</p>
      {list === null && (
        <p className="note" role="alert">
          {t('appList.loadFailed')}
        </p>
      )}
      {list && (
        <div className="settings-row">
          <span>{describe(list, t, format)}</span>
          <button type="button" className="button" disabled={fetching} onClick={fetchNow}>
            {t(fetching ? 'appList.fetching' : 'appList.fetch')}
          </button>
        </div>
      )}
      {fetching ? (
        <p className="note" role="status">
          {t('appList.fetchingNote')}
        </p>
      ) : (
        notice && (
          <p className="note" role={notice.role}>
            {t(notice.message)}
          </p>
        )
      )}
    </section>
  );
}

/**
 * How much of the list is stored, and since when. A list once fetched to its
 * end stays whole while a later fetch is unfinished.
 */
function describe(list: SteamAppList, t: ReturnType<typeof useT>, format: ReturnType<typeof useFormat>): string {
  const count = format.count(list.titles);
  if (list.fetchedAt !== null) return t('appList.fetched', { count, time: format.dateTime(list.fetchedAt) });
  return t(list.unfinished ? 'appList.unfinished' : 'appList.never', { count });
}
