import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { AboutButton } from './About';
import { Collections } from './Collections';
import { FavoritesEntry, FavoriteToggle, NoFavorites, useFavoriteMarking } from './Favorites';
import { GameCollections } from './GameCollections';
import { GameDetail } from './GameDetail';
import { useFormat, useT, type Lang, type MessageKey } from './i18n';
import { LanguageProvider } from './language';
import { LanguageSettings } from './settings/LanguageSettings';
import { AddLocalGameButton } from './library/AddLocalGame';
import { AddFromFolderButton } from './library/AddFromFolder';
import { HiddenGamesPage } from './library/HiddenGames';
import { useSessionChanges } from './library/Sessions';
import { LocalGameActions } from './library/LocalGameActions';
import type { Collection, Database, Game, Operations, SteamLogin, SyncOutcome } from './operations';
import { ThemeSettings } from './settings/ThemeSettings';
import { ThemeProvider } from './theme';
import { GameList, LibraryLayoutSwitch, useLibraryLayout } from './library/LibraryLayout';
import { SteamAppListSettings } from './settings/SteamAppListSettings';
import { isFiltering, LibraryToolbar, NO_FILTER, NoMatch, useFilteredLibrary } from './library/LibraryFilter';
import { WindowSettings } from './settings/WindowSettings';
import { Tray } from './Tray';
import { SortChoice, useLibrarySort } from './library/LibrarySort';
import { StatusBadge } from './library/Status';
import { ProgressBadge } from './library/Progress';
import { ExportSettings } from './settings/ExportSettings';
import { ImportReloadProvider, useImportReload } from './settings/importReload';
import { LogSettings } from './settings/LogSettings';
import { StatisticsPage } from './statistics/StatisticsPage';
import { GameActionButton, useGameAction } from './library/GameAction';
import { LimitedModeHint } from './LimitedModeHint';
import { UnreadableDatabasePage } from './UnreadableDatabase';

type View = 'library' | 'statistics' | 'settings';

const VIEWS: { view: View; label: MessageKey }[] = [
  { view: 'library', label: 'nav.library' },
  { view: 'statistics', label: 'nav.statistics' },
  { view: 'settings', label: 'nav.settings' },
];

/** The whole interface; `lang` is the system language, shown until the user chooses another. */
export function App({
  operations,
  lang,
  database = { state: 'open' },
}: {
  operations: Operations;
  lang: Lang;
  /** Whether the store is open, as `operations.database()` answered before the interface started. */
  database?: Database;
}) {
  if (database.state === 'unreadable') {
    return <UnreadableDatabasePage operations={operations} lang={lang} reason={database.reason} path={database.path} />;
  }
  return (
    <ImportReloadProvider>
      <LanguageProvider operations={operations} system={lang}>
        <ThemeProvider operations={operations}>
          <Shell operations={operations} />
        </ThemeProvider>
      </LanguageProvider>
    </ImportReloadProvider>
  );
}

function Shell({ operations }: { operations: Operations }) {
  const t = useT();
  // After an import the interface starts over where the import was made.
  const { imported } = useImportReload();
  const [view, setView] = useState<View>(imported ? 'settings' : 'library');
  // How many Syncs succeeded while the app runs; each one changes the stored Library.
  const [syncs, setSyncs] = useState(0);
  // How often the user chose a Steam Account while the app runs; each choice changes whose Sync and Library are shown.
  const [accountChoices, setAccountChoices] = useState(0);
  // Each Session that begins or ends changes the stored Library, as a Sync does.
  const changes = syncs + useSessionChanges(operations);
  // Whether the Library shows only the Favorites, chosen in the sidebar.
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  // How many Favorite marks the core took while the app runs; each one changes the stored Library.
  const [favoriteMarks, setFavoriteMarks] = useState(0);
  // How often Games were hidden, unhidden or removed here; each time changes what the sidebar counts.
  const [libraryEdits, setLibraryEdits] = useState(0);
  // The Collection the Library is narrowed to, chosen in the sidebar.
  const [chosen, setChosen] = useState<Collection>();
  // How often Collections or what is in them changed while the app runs; each change may change the counts.
  const [collectionChanges, setCollectionChanges] = useState(0);
  const collectionsChanged = () => setCollectionChanges((n) => n + 1);
  // How many Syncs ended while the app runs, successful or not; each one may end or change Limited Mode.
  const [syncsEnded, setSyncsEnded] = useState(0);
  // How often a Sync was asked for outside the sidebar; Sync now runs it.
  const [syncRequests, setSyncRequests] = useState(0);
  // Whether the settings open at the section of the Steam Web API key.
  const [toKeySettings, setToKeySettings] = useState(false);
  const show = (next: View) => {
    setToKeySettings(false);
    setView(next);
  };
  // The Game whose detail page the Library opens with, chosen on another page such as the Statistics.
  const [opening, setOpening] = useState<Game>();
  useEffect(() => {
    if (view !== 'library') setOpening(undefined);
  }, [view]);
  const openGame = (game: Game) => {
    setOpening(game);
    show('library');
    setFavoritesOnly(false);
    setChosen(undefined);
  };

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
              aria-current={
                view === v.view && !(v.view === 'library' && (favoritesOnly || chosen)) ? 'page' : undefined
              }
              onClick={() => {
                show(v.view);
                setFavoritesOnly(false);
                if (v.view === 'library') setChosen(undefined);
              }}
            >
              {t(v.label)}
            </button>
          ))}
        </nav>
        <FavoritesEntry
          operations={operations}
          changes={syncs + favoriteMarks + libraryEdits}
          active={view === 'library' && favoritesOnly}
          onOpen={() => {
            setView('library');
            setFavoritesOnly(true);
            setChosen(undefined);
          }}
        />
        <Collections
          operations={operations}
          changes={syncs + accountChoices + collectionChanges + libraryEdits}
          chosen={chosen}
          current={view === 'library'}
          onChoose={(collection) => {
            setChosen(collection);
            setFavoritesOnly(false);
            setView('library');
          }}
          onChosenChanged={setChosen}
          onChanged={collectionsChanged}
        />
        {/* Starts over for each chosen account: shows its last Sync, never one of the account chosen before. */}
        <SyncStatus
          key={accountChoices}
          operations={operations}
          requests={syncRequests}
          onSynced={() => setSyncs((n) => n + 1)}
          onEnded={() => setSyncsEnded((n) => n + 1)}
        />
        <AboutButton operations={operations} />
      </aside>
      <main className="main">
        {view === 'library' && (
          <LibraryView
            operations={operations}
            syncs={changes}
            favoritesOnly={favoritesOnly}
            favoriteMarks={favoriteMarks}
            onFavoriteMarked={() => setFavoriteMarks((n) => n + 1)}
            onLibraryEdited={() => setLibraryEdits((n) => n + 1)}
            collection={chosen}
            collectionChanges={collectionChanges}
            onCollectionsChanged={collectionsChanged}
            opening={opening}
            hint={
              <LimitedModeHint
                key={accountChoices}
                operations={operations}
                syncsEnded={syncsEnded}
                onOpenKeySettings={() => {
                  setView('settings');
                  setToKeySettings(true);
                }}
                onSync={() => setSyncRequests((n) => n + 1)}
              />
            }
          />
        )}
        {view === 'statistics' && <StatisticsPage operations={operations} syncs={changes} onOpen={openGame} />}
        {view === 'settings' && (
          <SettingsView
            operations={operations}
            toKeySettings={toKeySettings}
            onAccountChosen={() => setAccountChoices((n) => n + 1)}
          />
        )}
      </main>
      <Tray operations={operations} />
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
function SyncStatus({
  operations,
  requests,
  onSynced,
  onEnded,
}: {
  operations: Operations;
  /** How often a Sync was asked for elsewhere; each new one runs as Sync now does. */
  requests: number;
  onSynced: () => void;
  /** Hears each Sync shown here end, successful or not. */
  onEnded: () => void;
}) {
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
    onEnded();
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

  // The requests this view has answered; one made before it appeared is not its to run.
  const answered = useRef(requests);
  useEffect(() => {
    if (requests === answered.current) return;
    answered.current = requests;
    sync();
  });

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

function LibraryView({
  operations,
  syncs,
  favoritesOnly,
  favoriteMarks,
  onFavoriteMarked,
  onLibraryEdited,
  collection,
  collectionChanges,
  onCollectionsChanged,
  opening,
  hint,
}: {
  operations: Operations;
  syncs: number;
  /** Whether only the Favorites are shown. */
  favoritesOnly: boolean;
  /** Changes with each Favorite mark the core took. */
  favoriteMarks: number;
  onFavoriteMarked: () => void;
  /** Hears each Game hidden, unhidden or removed here, which changes what the sidebar counts. */
  onLibraryEdited: () => void;
  /** The Collection chosen in the sidebar: the Library shows only its Games. */
  collection?: Collection;
  /** Changes whenever Collections or what is in them changed. */
  collectionChanges: number;
  /** Hears each change of what is in a Collection made on a detail page. */
  onCollectionsChanged: () => void;
  /** The Game whose detail page shows when the Library opens. */
  opening?: Game;
  /** The Limited Mode hint, shown above the Library. */
  hint: ReactNode;
}) {
  const t = useT();
  // How many Local Games were added, edited or removed here; each one changes the stored Library.
  const [additions, setAdditions] = useState(0);
  // How often the user set a Status or a Manual Progress while the app runs; each one changes the stored Library.
  const [gameChanges, setGameChanges] = useState(0);
  // How often a Game was hidden or unhidden here; each time changes the Library.
  const [hidings, setHidings] = useState(0);
  // A change of what is in a Collection changes which Games a chosen Collection shows.
  const changes = syncs + additions + gameChanges + hidings + favoriteMarks + collectionChanges;
  const changed = () => setAdditions((n) => n + 1);
  // The title of the Local Game removed last, which the Library then confirms.
  const [removed, setRemoved] = useState<string>();
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
  const librarySort = useLibrarySort(operations);
  // What the toolbar sets, narrowed to the Favorites or the Collection the sidebar asks for.
  const chosenId = collection?.id;
  const narrowed = useMemo(
    () => (chosenId === undefined ? { ...filter, favoritesOnly } : { ...filter, favoritesOnly, collection: chosenId }),
    [filter, favoritesOnly, chosenId],
  );
  const filtered = useFilteredLibrary(operations, narrowed, librarySort.sort, library, changes);
  const games = filtered.state === 'ready' ? filtered.games : undefined;
  const failure: MessageKey | undefined =
    fromSteam.state !== 'failed'
      ? undefined
      : stored.state === 'failed'
        ? 'library.loadFailed'
        : 'library.readFailed';

  // Launch or Install, and why the last of them did not work, if it did not.
  const action = useGameAction(operations);

  const favorite = useFavoriteMarking(operations, onFavoriteMarked);
  const launchNotice = (
    <>
      {action.problem && (
        <p className="note" role="alert">
          {t(action.problem.message, { title: action.problem.title })}
        </p>
      )}
      {favorite.notice}
    </>
  );

  // How often each Game's Cover changed, so its image loads again.
  const [coverRevisions, setCoverRevisions] = useState<Record<string, number>>({});
  const coverUrl = (game: Game) => operations.coverUrl(game, coverRevisions[game.id]);
  const coverChanged = (game: Game) =>
    setCoverRevisions((revisions) => ({ ...revisions, [game.id]: (revisions[game.id] ?? 0) + 1 }));

  // The Game whose detail page is open. The Library stays mounted behind it,
  // so going back finds it as it was.
  const [opened, setOpened] = useState<Game | undefined>(opening);
  // Whether the Hidden Games are shown in place of the Library, which stays mounted behind them too.
  const [showingHidden, setShowingHidden] = useState(false);
  // The Game hidden last from its detail page, for the notice in the Library.
  const [hiddenLast, setHiddenLast] = useState<Game>();
  const libraryView = useRef<HTMLDivElement>(null);
  // Where the Library was scrolled to and what had the focus when a Game or the Hidden Games were opened.
  const left = useRef<{ scrollTop: number; focus: Element | null }>(undefined);
  const leave = () => {
    left.current = { scrollTop: libraryView.current?.closest('main')?.scrollTop ?? 0, focus: document.activeElement };
    action.clear();
    setHiddenLast(undefined);
    setRemoved(undefined);
  };
  const open = (game: Game) => {
    leave();
    setOpened(game);
  };
  const showHidden = () => {
    leave();
    setShowingHidden(true);
  };
  const back = () => {
    action.clear();
    setOpened(undefined);
    setShowingHidden(false);
  };
  const hidden = (game: Game) => {
    back();
    setHiddenLast(game);
    setHidings((n) => n + 1);
    onLibraryEdited();
  };
  // Choosing another Collection, the Favorites or the whole Library shows that list, not the Game
  // or the Hidden Games opened before; the Game the Library opens with stays open.
  const shownList = useRef({ chosenId, favoritesOnly });
  useEffect(() => {
    if (shownList.current.chosenId === chosenId && shownList.current.favoritesOnly === favoritesOnly) return;
    shownList.current = { chosenId, favoritesOnly };
    setOpened(undefined);
    setShowingHidden(false);
  }, [chosenId, favoritesOnly]);
  useLayoutEffect(() => {
    const main = libraryView.current?.closest('main');
    if (!main) return;
    if (opened || showingHidden) {
      main.scrollTop = 0;
    } else if (left.current) {
      if (left.current.focus instanceof HTMLElement) left.current.focus.focus({ preventScroll: true });
      main.scrollTop = left.current.scrollTop;
      left.current = undefined;
    }
  }, [opened, showingHidden]);

  return (
    <>
      <div ref={libraryView} hidden={opened !== undefined || showingHidden}>
        <Page title={favoritesOnly ? 'favorites.title' : 'nav.library'} heading={collection?.name} labelId="library-title">
          {hint}
          <div className="library-actions">
            <AddLocalGameButton operations={operations} onAdded={changed} />
            <AddFromFolderButton operations={operations} onAdded={changed} />
            <button type="button" className="button" onClick={showHidden}>
              {t('hidden.title')}
            </button>
          </div>
          {hiddenLast && (
            <p className="note" role="status">
              {t('hidden.hidden', { title: hiddenLast.title })}
            </p>
          )}
          {removed !== undefined && (
            <p className="note" role="status">
              {t('editGame.removed', { title: removed })}
            </p>
          )}
          <LibraryToolbar filter={filter} onChange={setFilter}>
            <SortChoice sort={librarySort.sort} choose={librarySort.choose} />
            <LibraryLayoutSwitch choice={layout} />
          </LibraryToolbar>
          {librarySort.notKept && (
            <p className="note" role="alert">
              {t('sort.notKept')}
            </p>
          )}
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
          {!isFiltering(narrowed) && fromSteam.state === 'ready' && games?.length === 0 && (
            <p className="note">{t('library.empty')}</p>
          )}
          {favoritesOnly && !isFiltering(filter) && games?.length === 0 && <NoFavorites />}
          {collection && !isFiltering(filter) && games?.length === 0 && <p className="note">{t('collections.empty')}</p>}
          {isFiltering(filter) && games?.length === 0 && <NoMatch onReset={() => setFilter(NO_FILTER)} />}
          {games && games.length > 0 && layout.layout === 'coverGrid' && (
            <CoverGrid
              games={games}
              coverUrl={coverUrl}
              onPlay={action.play}
              onInstall={action.install}
              onOpen={open}
              onFavorite={favorite.toggle}
            />
          )}
          {games && games.length > 0 && layout.layout === 'list' && (
            <GameList
              games={games}
              cover={(game) => <Cover title={game.title} src={coverUrl(game)} />}
              onPlay={action.play}
              onInstall={action.install}
              onOpen={open}
              onFavorite={favorite.toggle}
            />
          )}
        </Page>
      </div>
      {opened && (
        <GameDetail
          key={opened.id}
          operations={operations}
          game={opened}
          syncs={syncs + additions + favoriteMarks}
          cover={(game) => <Cover key={game.id} title={game.title} src={coverUrl(game)} />}
          collections={
            <GameCollections
              operations={operations}
              gameId={opened.id}
              title={opened.title}
              changes={collectionChanges}
              onChanged={onCollectionsChanged}
            />
          }
          notice={launchNotice}
          onPlay={action.play}
          onInstall={action.install}
          onChanged={() => setGameChanges((count) => count + 1)}
          onFavorite={favorite.toggle}
          onBack={back}
          onHidden={hidden}
          onCoverChanged={coverChanged}
          actions={(game) =>
            game.source === 'local' && (
              <LocalGameActions
                operations={operations}
                game={game}
                onEdited={() => {
                  changed();
                  // A new executable may bring a new icon as the Cover.
                  coverChanged(game);
                }}
                onRemoved={(gone) => {
                  changed();
                  onLibraryEdited();
                  back();
                  setRemoved(gone.title);
                }}
              />
            )
          }
        />
      )}
      {showingHidden && (
        <HiddenGamesPage operations={operations} onUnhidden={() => {
            setHidings((n) => n + 1);
            onLibraryEdited();
          }} onBack={back} />
      )}
    </>
  );
}

function CoverGrid({
  games,
  coverUrl,
  onPlay,
  onInstall,
  onOpen,
  onFavorite,
}: {
  games: Game[];
  coverUrl: Operations['coverUrl'];
  onPlay: (game: Game) => void;
  /** Starts the installation of a Steam Game that is not Installed. */
  onInstall: (game: Game) => void;
  /** Opens the Game's detail page. */
  onOpen: (game: Game) => void;
  /** Marks the Game as a Favorite, or unmarks it. */
  onFavorite: (game: Game) => void;
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
            <GameActionButton game={game} className="play" onPlay={onPlay} onInstall={onInstall} />
            <FavoriteToggle game={game} onToggle={onFavorite} />
          </div>
          <h2 className="game-title">{game.title}</h2>
          <StatusBadge status={game.status} />
          <ProgressBadge progress={game.progress} />
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
  // The address that failed to load; a new one, after the Cover changed, is tried again.
  const [failedSrc, setFailedSrc] = useState<string>();
  const label = t('library.cover', { title });

  if (failedSrc === src) {
    return (
      <div className="cover cover-placeholder" role="img" aria-label={label}>
        <span>{title}</span>
      </div>
    );
  }
  return (
    <img className="cover" src={src} alt={label} loading="lazy" decoding="async" onError={() => setFailedSrc(src)} />
  );
}

function SettingsView({
  operations,
  toKeySettings,
  onAccountChosen,
}: {
  operations: Operations;
  /** Whether to open at the section of the Steam Web API key. */
  toKeySettings: boolean;
  onAccountChosen: () => void;
}) {
  useEffect(() => {
    if (toKeySettings) document.getElementById('settings-key')?.scrollIntoView?.({ block: 'start' });
  }, [toKeySettings]);
  return (
    <Page title="nav.settings">
      <SteamSettings operations={operations} onAccountChosen={onAccountChosen} />
      <SteamWebApiKeySettings operations={operations} />
      <ThemeSettings />
      <LanguageSettings />
      <WindowSettings operations={operations} />
      <SteamAppListSettings operations={operations} />
      <ExportSettings operations={operations} />
      <LogSettings operations={operations} />
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

  const detected = steam.state === 'ready' && steam.value.state === 'found' ? steam.value.logins : [];
  // A SteamID in use that no radio button shows: entered by hand, or a Steam Login that left this computer.
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
          {steam.value.loginsUnreadable ? (
            <p className="note">{t('steam.accountsUnreadable')}</p>
          ) : (
            steam.value.logins.length === 0 && <p className="note">{t('steam.noAccounts')}</p>
          )}
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
            <AccountChoice logins={detected} chosen={account.value} onChoose={choose} />
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

/** The Steam Logins to choose the Steam Account from. */
function AccountChoice({
  logins,
  chosen,
  onChoose,
}: {
  logins: SteamLogin[];
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
        {logins.map((login) => (
          <li key={login.steamId} className="settings-row">
            <label>
              <input
                type="radio"
                name="steam-account"
                value={login.steamId}
                checked={login.steamId === chosen}
                onChange={() => onChoose(login.steamId)}
              />
              {login.displayName}
            </label>
            {login.mostRecent && <span className="muted">{t('steam.mostRecent')}</span>}
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Entering a SteamID by hand, for when there is no Steam Login on this computer. */
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

/**
 * A page under its title; `heading`, when given, is shown in place of the
 * title's text. With `labelId` the page is a region named by its heading,
 * whose id is `labelId`.
 */
function Page({
  title,
  heading,
  labelId,
  children,
}: {
  title: MessageKey;
  heading?: string;
  labelId?: string;
  children: ReactNode;
}) {
  const t = useT();
  return (
    <section className="page" aria-labelledby={labelId}>
      <h1 id={labelId}>{heading ?? t(title)}</h1>
      {children}
    </section>
  );
}
