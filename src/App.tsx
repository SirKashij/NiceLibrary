import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Collections } from './Collections';
import { GameDetail } from './GameDetail';
import { I18nProvider, useFormat, useT, type Lang, type MessageKey } from './i18n';
import { AddLocalGameButton } from './library/AddLocalGame';
import type { Game, Operations, SteamAccount, SyncOutcome } from './operations';
import { ThemeSettings } from './settings/ThemeSettings';
import { ThemeProvider } from './theme';
import { GameList, LibraryLayoutSwitch, useLibraryLayout } from './library/LibraryLayout';
import { isFiltering, LibraryToolbar, NO_FILTER, NoMatch, useFilteredLibrary } from './library/LibraryFilter';

type View = 'library' | 'statistics' | 'settings';

const VIEWS: { view: View; label: MessageKey }[] = [
  { view: 'library', label: 'nav.library' },
  { view: 'statistics', label: 'nav.statistics' },
  { view: 'settings', label: 'nav.settings' },
];

export function App({ operations, lang }: { operations: Operations; lang: Lang }) {
  return (
    <I18nProvider lang={lang}>
      <ThemeProvider operations={operations}>
        <Shell operations={operations} />
      </ThemeProvider>
    </I18nProvider>
  );
}

function Shell({ operations }: { operations: Operations }) {
  const t = useT();
  const [view, setView] = useState<View>('library');
  // How many Syncs succeeded while the app runs; each one changes the stored Library.
  const [syncs, setSyncs] = useState(0);
  // How often the user chose a Steam Account while the app runs; each choice changes whose Sync and Library are shown.
  const [accountChoices, setAccountChoices] = useState(0);

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true" />
          {t('app.name')}
        </div>
        <nav aria-label={t('nav.main')}>
          {VIEWS.map((v) => (
            <button
              key={v.view}
              type="button"
              aria-current={view === v.view ? 'page' : undefined}
              onClick={() => setView(v.view)}
            >
              {t(v.label)}
            </button>
          ))}
        </nav>
        <Collections operations={operations} />
        {/* Starts over for each chosen account: shows its last Sync, never one of the account chosen before. */}
        <SyncStatus key={accountChoices} operations={operations} onSynced={() => setSyncs((n) => n + 1)} />
      </aside>
      <main className="main">
        {view === 'library' && <LibraryView operations={operations} syncs={syncs} />}
        {view === 'statistics' && <PlaceholderPage title="nav.statistics" note="statistics.empty" />}
        {view === 'settings' && (
          <SettingsView operations={operations} onAccountChosen={() => setAccountChoices((n) => n + 1)} />
        )}
      </main>
    </div>
  );
}

type Loaded<T> = { state: 'loading' } | { state: 'ready'; value: T } | { state: 'failed' };

/** Calls one operation when the view appears and tracks its answer. */
function useOperation<T>(operations: Operations, call: (operations: Operations) => Promise<T>): Loaded<T> {
  const [loaded, setLoaded] = useState<Loaded<T>>({ state: 'loading' });

  useEffect(() => {
    let active = true;
    call(operations).then(
      (value) => active && setLoaded({ state: 'ready', value }),
      () => active && setLoaded({ state: 'failed' }),
    );
    return () => {
      active = false;
    };
    // `call` only names which operation to run and is the same for the life of a view.
  }, [operations]);

  return loaded;
}

/** What a Sync that did not bring the Library answers, for the user. */
const SYNC_PROBLEMS: Record<Exclude<SyncOutcome['outcome'], 'synced'>, MessageKey> = {
  noKey: 'sync.noKey',
  noSteamAccount: 'sync.noSteamAccount',
  keyRejected: 'sync.keyRejected',
  gameDetailsPrivate: 'sync.gameDetailsPrivate',
  steamUnreachable: 'sync.steamUnreachable',
};

/** At the bottom of the sidebar: when the last successful Sync happened, and Sync now. */
function SyncStatus({ operations, onSynced }: { operations: Operations; onSynced: () => void }) {
  const t = useT();
  const format = useFormat();
  const loaded = useOperation(operations, (o) => o.lastSync());
  // The time of a Sync made here; until then what the core answered.
  const [syncedAt, setSyncedAt] = useState<number>();
  const last = syncedAt ?? (loaded.state === 'ready' ? loaded.value : null);
  const [syncing, setSyncing] = useState(false);
  const [problem, setProblem] = useState<MessageKey>();
  // Whether this view saw the running Sync start. One that started before is
  // for the Steam Account chosen before, so its answer is not shown here.
  const running = useRef(false);

  const begin = () => {
    running.current = true;
    setSyncing(true);
    setProblem(undefined);
  };
  /** Shows how a Sync ended, started here or by the core on its own. */
  const settle = (answer: SyncOutcome | 'failed') => {
    if (!running.current) return;
    running.current = false;
    setSyncing(false);
    if (answer === 'failed') {
      setProblem('sync.failed');
    } else if (answer.outcome === 'synced') {
      setSyncedAt(answer.at);
      onSynced();
    } else {
      setProblem(SYNC_PROBLEMS[answer.outcome]);
    }
  };
  // The latest render's handlers, for the listener that lives as long as the view.
  const handlers = useRef({ begin, settle });
  handlers.current = { begin, settle };

  useEffect(
    () =>
      operations.onAutomaticSync((event) => {
        if (event.state === 'started') handlers.current.begin();
        else handlers.current.settle(event.state === 'failed' ? 'failed' : event);
      }),
    [operations],
  );

  const sync = () => {
    if (syncing) return;
    begin();
    operations.sync().then(settle, () => settle('failed'));
  };

  return (
    <section className="sync" aria-label={t('sync.title')}>
      <p className="muted">{last === null ? t('sync.never') : t('sync.last', { time: format.dateTime(last) })}</p>
      <button type="button" className="button" disabled={syncing} onClick={sync}>
        {t(syncing ? 'sync.syncing' : 'sync.now')}
      </button>
      {problem && (
        <p className="note" role="alert">
          {t(problem)}
        </p>
      )}
    </section>
  );
}

function LibraryView({ operations, syncs }: { operations: Operations; syncs: number }) {
  const t = useT();
  // How many Local Games were added here; each one changes the stored Library.
  const [additions, setAdditions] = useState(0);
  const changes = syncs + additions;
  // The stored Library answers at once; reading Steam again replaces it.
  const stored = useOperation(operations, (o) => o.library());
  const fromSteam = useOperation(operations, (o) => o.readInstalledGames());
  // The Library as answered last after reading Steam or after a Sync, whichever came later.
  const [fresh, setFresh] = useState<Game[]>();
  useEffect(() => {
    if (fromSteam.state === 'ready') setFresh(fromSteam.value);
  }, [fromSteam]);
  useEffect(() => {
    if (changes === 0) return;
    let active = true;
    // A Library that cannot be loaded now stays as shown; the next answer replaces it.
    operations.library().then(
      (games) => active && setFresh(games),
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations, changes]);

  const library = fresh ?? (stored.state === 'ready' ? stored.value : undefined);
  const [filter, setFilter] = useState(NO_FILTER);
  const layout = useLibraryLayout(operations);
  const filtered = useFilteredLibrary(operations, filter, library);
  const games = filtered.state === 'ready' ? filtered.games : undefined;
  const failure: MessageKey | undefined =
    fromSteam.state !== 'failed'
      ? undefined
      : stored.state === 'failed'
        ? 'library.loadFailed'
        : 'library.readFailed';

  // Why the last Launch did not start its Game, if it did not.
  const [launchProblem, setLaunchProblem] = useState<{ message: MessageKey; title: string }>();
  // Only the answer to the latest Launch counts; an earlier one may answer later.
  const latestLaunch = useRef(0);
  const launch = (game: Game) => {
    const id = ++latestLaunch.current;
    const report = (problem?: { message: MessageKey; title: string }) => {
      if (id === latestLaunch.current) setLaunchProblem(problem);
    };
    operations.launch(game).then(
      ({ outcome }) => report(outcome === 'steamNotFound' ? { message: 'launch.steamNotFound', title: game.title } : undefined),
      () => report({ message: 'launch.failed', title: game.title }),
    );
  };

  const launchNotice = launchProblem && (
    <p className="note" role="alert">
      {t(launchProblem.message, { title: launchProblem.title })}
    </p>
  );

  // The Game whose detail page is open. The Library stays mounted behind it,
  // so going back finds it as it was.
  const [opened, setOpened] = useState<Game>();
  const libraryView = useRef<HTMLDivElement>(null);
  // Where the Library was scrolled to and what had the focus when a Game was opened.
  const left = useRef<{ scrollTop: number; focus: Element | null }>(undefined);
  const open = (game: Game) => {
    left.current = { scrollTop: libraryView.current?.closest('main')?.scrollTop ?? 0, focus: document.activeElement };
    setLaunchProblem(undefined);
    setOpened(game);
  };
  const back = () => {
    setLaunchProblem(undefined);
    setOpened(undefined);
  };
  useLayoutEffect(() => {
    const main = libraryView.current?.closest('main');
    if (!main) return;
    if (opened) {
      main.scrollTop = 0;
    } else if (left.current) {
      if (left.current.focus instanceof HTMLElement) left.current.focus.focus({ preventScroll: true });
      main.scrollTop = left.current.scrollTop;
      left.current = undefined;
    }
  }, [opened]);

  return (
    <>
      <div ref={libraryView} hidden={opened !== undefined}>
        <Page title="nav.library">
          <AddLocalGameButton operations={operations} onAdded={() => setAdditions((n) => n + 1)} />
          <LibraryToolbar filter={filter} onChange={setFilter}>
            <LibraryLayoutSwitch choice={layout} />
          </LibraryToolbar>
          {layout.saveFailed && (
            <p className="note" role="alert">
              {t('layout.saveFailed')}
            </p>
          )}
          {filtered.state === 'failed' && (
            <p className="note" role="alert">
              {t('filter.failed')}
            </p>
          )}
          {failure && (
            <p className="note" role="alert">
              {t(failure)}
            </p>
          )}
          {!opened && launchNotice}
          {!isFiltering(filter) && fromSteam.state === 'ready' && games?.length === 0 && (
            <p className="note">{t('library.empty')}</p>
          )}
          {isFiltering(filter) && games?.length === 0 && <NoMatch onReset={() => setFilter(NO_FILTER)} />}
          {games && games.length > 0 && layout.layout === 'coverGrid' && (
            <CoverGrid games={games} coverUrl={operations.coverUrl} onPlay={launch} onOpen={open} />
          )}
          {games && games.length > 0 && layout.layout === 'list' && (
            <GameList
              games={games}
              cover={(game) => <Cover title={game.title} src={operations.coverUrl(game)} />}
              onPlay={launch}
              onOpen={open}
            />
          )}
        </Page>
      </div>
      {opened && (
        <GameDetail
          key={opened.id}
          operations={operations}
          game={opened}
          syncs={syncs}
          cover={(game) => <Cover key={game.id} title={game.title} src={operations.coverUrl(game)} />}
          notice={launchNotice}
          onPlay={launch}
          onBack={back}
        />
      )}
    </>
  );
}

function CoverGrid({
  games,
  coverUrl,
  onPlay,
  onOpen,
}: {
  games: Game[];
  coverUrl: Operations['coverUrl'];
  onPlay: (game: Game) => void;
  /** Opens the Game's detail page. */
  onOpen: (game: Game) => void;
}) {
  const t = useT();
  return (
    <ul className="games" aria-label={t('library.games')}>
      {games.map((game) => (
        <li key={game.id} className="game" data-installed={game.installed}>
          <div className="cover-frame">
            <button
              type="button"
              className="open-game"
              aria-label={t('library.openGame', { title: game.title })}
              onClick={() => onOpen(game)}
            >
              <Cover title={game.title} src={coverUrl(game)} />
            </button>
            {game.installed && (
              <button
                type="button"
                className="play"
                aria-label={t('library.playGame', { title: game.title })}
                onClick={() => onPlay(game)}
              >
                {t('library.play')}
              </button>
            )}
          </div>
          <h2 className="game-title">{game.title}</h2>
          <Played game={game} />
          {!game.installed && <span className="muted">{t('library.notInstalled')}</span>}
        </li>
      ))}
    </ul>
  );
}

/** A Game's Playtime and Last Played, once they are known. */
function Played({ game }: { game: Game }) {
  const t = useT();
  const format = useFormat();
  const { playtimeMinutes: minutes, lastPlayed } = game;
  if (minutes === null) return null;
  const text =
    lastPlayed !== null
      ? t('library.playtimeAndLastPlayed', { time: format.playtime(minutes), date: format.date(lastPlayed) })
      : minutes > 0
        ? t('library.playtime', { time: format.playtime(minutes) })
        : t('library.neverPlayed');
  return <span className="game-played muted">{text}</span>;
}

/** A Game's Cover, or a neutral placeholder with its title when there is none or it cannot be loaded. */
function Cover({ title, src }: { title: string; src: string }) {
  const t = useT();
  const [failed, setFailed] = useState(false);
  const label = t('library.cover', { title });

  if (failed) {
    return (
      <div className="cover cover-placeholder" role="img" aria-label={label}>
        <span>{title}</span>
      </div>
    );
  }
  return (
    <img className="cover" src={src} alt={label} loading="lazy" decoding="async" onError={() => setFailed(true)} />
  );
}

function SettingsView({ operations, onAccountChosen }: { operations: Operations; onAccountChosen: () => void }) {
  return (
    <Page title="nav.settings">
      <SteamSettings operations={operations} onAccountChosen={onAccountChosen} />
      <SteamWebApiKeySettings operations={operations} />
      <ThemeSettings />
    </Page>
  );
}

/** The Steam installation and the Steam Account the app works with; `onAccountChosen` hears each account the core accepted. */
function SteamSettings({ operations, onAccountChosen }: { operations: Operations; onAccountChosen: () => void }) {
  const t = useT();
  const steam = useOperation(operations, (o) => o.steamInstallation());
  const stored = useOperation(operations, (o) => o.steamAccount());
  // The account chosen here, once the user chose; until then the stored one.
  const [chosen, setChosen] = useState<{ value: string | null }>();
  const account: Loaded<string | null> = chosen ? { state: 'ready', value: chosen.value } : stored;
  // Why the last choice did not take, if it did not.
  const [problem, setProblem] = useState<MessageKey>();
  // Only the answer to the latest choice counts; an earlier one may answer later.
  const latestChoice = useRef(0);

  const choose = (steamId: string) => {
    const id = ++latestChoice.current;
    const before = account.state === 'ready' ? account.value : null;
    const settle = (value: string | null, message?: MessageKey) => {
      if (id !== latestChoice.current) return;
      setChosen({ value });
      setProblem(message);
    };
    // Shown at once; taken back when the core does not accept it.
    setChosen({ value: steamId });
    operations.chooseSteamAccount(steamId).then(
      ({ outcome }) => {
        if (outcome !== 'chosen') return settle(before, 'steam.invalidSteamId');
        onAccountChosen();
        // The core decides how a SteamID entered by hand is written.
        return operations.steamAccount().then(settle, () => settle(steamId));
      },
      () => settle(before, 'steam.saveFailed'),
    );
  };

  const detected = steam.state === 'ready' && steam.value.state === 'found' ? steam.value.accounts : [];
  // A SteamID in use that no radio button shows: entered by hand, or an account that left this computer.
  const inUseElsewhere =
    account.state === 'ready' && account.value !== null && !detected.some((a) => a.steamId === account.value)
      ? account.value
      : undefined;

  return (
    <section className="settings-section" aria-labelledby="settings-steam">
      <h2 id="settings-steam">{t('settings.steam')}</h2>
      {steam.state === 'failed' && (
        <p className="note" role="alert">
          {t('steam.loadFailed')}
        </p>
      )}
      {steam.state === 'ready' && steam.value.state === 'notFound' && <p className="note">{t('steam.notFound')}</p>}
      {steam.state === 'ready' && steam.value.state === 'found' && (
        <>
          <div className="settings-row">
            <span>{t('steam.found')}</span>
            <span className="muted">{steam.value.path}</span>
          </div>
          {steam.value.accounts.length === 0 && <p className="note">{t('steam.noAccounts')}</p>}
        </>
      )}
      {account.state === 'failed' && (
        <p className="note" role="alert">
          {t('steam.accountLoadFailed')}
        </p>
      )}
      {steam.state === 'ready' && account.state === 'ready' && (
        <>
          {detected.length > 0 ? (
            <AccountChoice accounts={detected} chosen={account.value} onChoose={choose} />
          ) : (
            <SteamIdForm chosen={account.value} onChoose={choose} />
          )}
          {inUseElsewhere && (
            <p className="note" role="status">
              {t('steam.usesSteamId', { steamId: inUseElsewhere })}
            </p>
          )}
          {problem && (
            <p className="note" role="alert">
              {t(problem)}
            </p>
          )}
        </>
      )}
    </section>
  );
}

/** The Steam Accounts on this computer, one of them the account the Library shows. */
function AccountChoice({
  accounts,
  chosen,
  onChoose,
}: {
  accounts: SteamAccount[];
  chosen: string | null;
  onChoose: (steamId: string) => void;
}) {
  const t = useT();
  return (
    <div role="radiogroup" aria-labelledby="steam-account-choice" className="account-choice">
      <p id="steam-account-choice" className="muted">
        {t('steam.chooseAccount')}
      </p>
      <ul className="accounts" aria-label={t('steam.accounts')}>
        {accounts.map((account) => (
          <li key={account.steamId} className="settings-row">
            <label>
              <input
                type="radio"
                name="steam-account"
                value={account.steamId}
                checked={account.steamId === chosen}
                onChange={() => onChoose(account.steamId)}
              />
              {account.displayName}
            </label>
            {account.mostRecent && <span className="muted">{t('steam.mostRecent')}</span>}
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Entering a SteamID by hand, for when no Steam Account is found on this computer. */
function SteamIdForm({ chosen, onChoose }: { chosen: string | null; onChoose: (steamId: string) => void }) {
  const t = useT();
  const [draft, setDraft] = useState(chosen ?? '');
  return (
    <form
      className="steam-id-form"
      onSubmit={(event) => {
        event.preventDefault();
        onChoose(draft);
      }}
    >
      <label htmlFor="steam-id">{t('steam.steamId')}</label>
      <div className="field-row">
        <input
          id="steam-id"
          type="text"
          inputMode="numeric"
          autoComplete="off"
          spellCheck={false}
          aria-describedby="steam-id-hint"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
        <button type="submit" className="button">
          {t('steam.useSteamId')}
        </button>
      </div>
      <p id="steam-id-hint" className="muted">
        {t('steam.steamIdHint')}
      </p>
    </form>
  );
}

/** A message under a setting: `status` for what went well, `alert` for what did not. */
type Notice = { role: 'status' | 'alert'; message: MessageKey };

/**
 * The Steam Web API key: a guided step for creating and entering one, and
 * whether one is saved, with replacing and removing it. The key itself
 * never comes back from the core.
 */
function SteamWebApiKeySettings({ operations }: { operations: Operations }) {
  const t = useT();
  const loaded = useOperation(operations, (o) => o.steamWebApiKeyStored());
  // Whether a key is saved after a change made here; until then what the core answered.
  const [changed, setChanged] = useState<boolean>();
  const stored = changed ?? (loaded.state === 'ready' ? loaded.value : undefined);
  const [replacing, setReplacing] = useState(false);
  const [checking, setChecking] = useState(false);
  const [notice, setNotice] = useState<Notice>();
  const [draft, setDraft] = useState('');

  const settle = (saved: boolean, message: MessageKey) => {
    setChanged(saved);
    setReplacing(false);
    setDraft('');
    setNotice({ role: 'status', message });
  };

  const check = () => {
    if (checking) return;
    setChecking(true);
    setNotice(undefined);
    operations
      .storeSteamWebApiKey(draft)
      .then(
        ({ outcome }) =>
          outcome === 'stored'
            ? settle(true, 'key.confirmed')
            : setNotice({ role: 'alert', message: outcome === 'invalid' ? 'key.invalid' : 'key.steamUnreachable' }),
        () => setNotice({ role: 'alert', message: 'key.saveFailed' }),
      )
      .finally(() => setChecking(false));
  };

  const remove = () => {
    setNotice(undefined);
    operations.removeSteamWebApiKey().then(
      () => settle(false, 'key.removed'),
      () => setNotice({ role: 'alert', message: 'key.removeFailed' }),
    );
  };

  const openPage = () => {
    operations.openSteamWebApiKeyPage().catch(() => setNotice({ role: 'alert', message: 'key.pageFailed' }));
  };

  const cancel = () => {
    setReplacing(false);
    setDraft('');
    setNotice(undefined);
  };

  return (
    <section className="settings-section" aria-labelledby="settings-key">
      <h2 id="settings-key">{t('settings.key')}</h2>
      <p className="note">{t('key.why')}</p>
      {loaded.state === 'failed' && stored === undefined && (
        <p className="note" role="alert">
          {t('key.loadFailed')}
        </p>
      )}
      {stored !== undefined && (
        <div className="settings-row">
          <span>{t(stored ? 'key.stored' : 'key.notStored')}</span>
          {stored && !replacing && (
            <span className="settings-actions">
              <button type="button" className="button" onClick={() => setReplacing(true)}>
                {t('key.replace')}
              </button>
              <button type="button" className="button" onClick={remove}>
                {t('key.remove')}
              </button>
            </span>
          )}
        </div>
      )}
      {stored !== undefined && (!stored || replacing) && (
        <form
          className="key-form"
          onSubmit={(event) => {
            event.preventDefault();
            check();
          }}
        >
          <p id="key-steps" className="muted">
            {t('key.steps')}
          </p>
          <ol className="key-steps" aria-labelledby="key-steps">
            <li>
              {t('key.step1')}
              <span className="key-page">
                <button type="button" className="button" onClick={openPage}>
                  {t('key.openPage')}
                </button>
                <span className="muted">steamcommunity.com/dev/apikey</span>
              </span>
            </li>
            <li>{t('key.step2')}</li>
            <li>{t('key.step3')}</li>
          </ol>
          <label htmlFor="steam-web-api-key">{t('key.field')}</label>
          <div className="field-row">
            <input
              id="steam-web-api-key"
              type="password"
              autoComplete="off"
              spellCheck={false}
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
            />
            <button type="submit" className="button" disabled={checking}>
              {t('key.check')}
            </button>
            {replacing && (
              <button type="button" className="button" onClick={cancel}>
                {t('key.cancel')}
              </button>
            )}
          </div>
        </form>
      )}
      {checking ? (
        <p className="note" role="status">
          {t('key.checking')}
        </p>
      ) : (
        notice && (
          <p className="note" role={notice.role}>
            {t(notice.message)}
          </p>
        )
      )}
      <div className="key-data" role="note" aria-labelledby="key-data-title">
        <h3 id="key-data-title">{t('key.dataTitle')}</h3>
        <p className="muted">{t('key.data')}</p>
      </div>
    </section>
  );
}

function PlaceholderPage({ title, note }: { title: MessageKey; note: MessageKey }) {
  const t = useT();
  return (
    <Page title={title}>
      <p className="note">{t(note)}</p>
    </Page>
  );
}

function Page({ title, children }: { title: MessageKey; children: ReactNode }) {
  const t = useT();
  return (
    <section className="page">
      <h1>{t(title)}</h1>
      {children}
    </section>
  );
}
