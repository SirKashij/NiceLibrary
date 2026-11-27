// The interface talks to the application core only through these operations.
// In the app they are Tauri commands; tests hand the interface a fake.
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type Source = 'steam' | 'local';

/** The person's own judgement of where they stand with a Game. Mirrors `Status` in crates/core. */
export type Status = 'unplayed' | 'playing' | 'completed' | 'dropped';

/** One playable title in the Library. Mirrors `Game` in crates/core. */
export interface Game {
  /** Unique across Sources: `steam:<app id>` for a Steam Game, `local:<number>` for a Local Game. */
  id: string;
  title: string;
  source: Source;
  installed: boolean;
  /**
   * Playtime in minutes; for a Steam Game Steam's figure, for a Local Game the
   * sum of its Sessions. `null` while not known, before the first Sync.
   */
  playtimeMinutes: number | null;
  /** Last Played, in seconds since the Unix epoch; `null` for never or not known. */
  lastPlayed: number | null;
  /**
   * Progress in percent, from 0 to 100, as the Game's Source works it out:
   * for a Local Game its Manual Progress, for a Steam Game the share of its
   * unlocked Achievements. `null` when the Game has none, such as a Steam Game
   * without Achievements.
   */
  progress: number | null;
  /** Set by hand only, `unplayed` until then; no Sync changes it. */
  status: Status;
  /** Whether the user marked the Game as a Favorite. */
  favorite: boolean;
}

/**
 * How setting or clearing a Local Game's Manual Progress went, when nothing
 * failed. Every outcome but `set` changed nothing. Mirrors `ManualProgress`
 * in crates/core.
 */
export type ManualProgress =
  | { outcome: 'set'; /** The Game as the Library shows it now. */ game: Game }
  | { outcome: 'outOfRange' }
  | { outcome: 'notALocalGame' };

/** How a Sync went, when nothing failed. Every outcome but `synced` changed nothing. Mirrors `SyncOutcome` in crates/core. */
export type SyncOutcome =
  | { outcome: 'synced'; /** Seconds since the Unix epoch. */ at: number }
  | { outcome: 'noKey' }
  | { outcome: 'noSteamAccount' }
  | { outcome: 'keyRejected' }
  | { outcome: 'gameDetailsPrivate' }
  | { outcome: 'steamUnreachable' };

/**
 * What happens in an automatic Sync, as it happens. Mirrors `AutomaticSync`
 * in crates/core; a round without a key never reaches the interface.
 */
export type AutomaticSync =
  | { state: 'started' }
  | ({ state: 'finished' } & SyncOutcome)
  | { state: 'failed'; reason: string };

/** A Steam Account that has logged in on this computer. Mirrors `SteamAccount` in crates/core. */
export interface SteamAccount {
  /** SteamID64, as text because it exceeds JavaScript's safe integers. */
  steamId: string;
  displayName: string;
  mostRecent: boolean;
}

/** The Steam installation on this computer. Mirrors `SteamInstallation` in crates/core. */
export type SteamInstallation =
  | { state: 'notFound' }
  | { state: 'found'; path: string; accounts: SteamAccount[] };

/** How choosing the Steam Account went, when nothing failed. Mirrors `SteamAccountChoice` in crates/core. */
export type SteamAccountChoice = { outcome: 'chosen' } | { outcome: 'invalidSteamId' };

/**
 * How a Launch went, when nothing failed. `executableNotFound` and
 * `workingDirectoryNotFound` are for a Local Game and started nothing.
 * Mirrors `Launch` in crates/core.
 */
export type LaunchOutcome =
  | { outcome: 'launched' }
  | { outcome: 'steamNotFound' }
  | { outcome: 'executableNotFound' }
  | { outcome: 'workingDirectoryNotFound' };

/** How a Local Game is launched, as the user set it. Mirrors `LaunchOptions` in crates/core. */
export interface LaunchOptions {
  /** The Launch Arguments as typed: separated by spaces, a part in double quotes is one argument. */
  arguments: string;
  /** The Working Directory; `null` for the executable's folder. */
  workingDirectory: string | null;
}

/** How storing the Steam Web API key went, when nothing failed. Mirrors `SteamWebApiKeyCheck` in crates/core. */
export type SteamWebApiKeyCheck = { outcome: 'stored' } | { outcome: 'invalid' } | { outcome: 'steamUnreachable' };

/**
 * What came of asking the user for a Local Game's executable, when nothing
 * failed. `chosen` carries the title the app proposes. Mirrors
 * `ExecutableChoice` in crates/core.
 */
export type ExecutableChoice =
  | { outcome: 'cancelled' }
  | { outcome: 'chosen'; executable: string; title: string }
  | { outcome: 'alreadyAdded'; /** The title the Game has in the Library. */ title: string };

/** How adding a Local Game went, when nothing failed. Every outcome but `added` changed nothing. Mirrors `AddLocalGame` in crates/core. */
export type AddLocalGame =
  | { outcome: 'added'; game: Game }
  | { outcome: 'alreadyAdded'; /** The title the Game has in the Library. */ title: string }
  | { outcome: 'noTitle' }
  | { outcome: 'notFound' };

/** A Local Game as the user edits it: its title and the path of its executable. Mirrors `LocalGame` in crates/core. */
export interface LocalGame {
  /** `local:<number>`. */
  id: string;
  title: string;
  executable: string;
}

/** How editing a Local Game went, when nothing failed. Every outcome but `edited` changed nothing. Mirrors `EditLocalGame` in crates/core. */
export type EditLocalGame =
  | { outcome: 'edited'; game: Game }
  | { outcome: 'alreadyAdded'; /** The title of the Local Game the executable belongs to. */ title: string }
  | { outcome: 'noTitle' }
  | { outcome: 'notFound' };

/** The theme the user chose; `system` follows the operating system's dark or light setting. Mirrors `Theme` in crates/core. */
export type Theme = 'system' | 'dark' | 'light';

/** A named group of Games the user created. Mirrors `Collection` in crates/core. */
export interface Collection {
  id: number;
  name: string;
  /** How many Games of the Library are in it. */
  gameCount: number;
}

/** How naming a Collection went, when nothing failed. Every outcome but `named` changed nothing. Mirrors `CollectionNaming` in crates/core. */
export type CollectionNaming = { outcome: 'named'; id: number } | { outcome: 'emptyName' } | { outcome: 'nameTaken' };

/** What is stored of the Steam App List. Mirrors `SteamAppList` in crates/core. */
export interface SteamAppList {
  /** How many titles are stored. */
  titles: number;
  /** When the last fetch that ran to its end ended, in seconds since the Unix epoch; `null` before the first. */
  fetchedAt: number | null;
  /** Whether a fetch stopped before its end; the next one continues it. */
  unfinished: boolean;
}

/**
 * How fetching the Steam App List went, when nothing failed. The pages
 * fetched before Steam stopped answering stay stored. Mirrors
 * `SteamAppListFetch` in crates/core.
 */
export type SteamAppListFetch =
  | { outcome: 'fetched'; /** Seconds since the Unix epoch. */ at: number }
  | { outcome: 'noKey' }
  | { outcome: 'keyRejected' }
  | { outcome: 'steamUnreachable' };

/**
 * What happens in a fetch of the Steam App List the core starts on its own.
 * Mirrors `AutomaticSteamAppListFetch` in crates/core; a round that fetches
 * nothing never reaches the interface.
 */
export type AutomaticSteamAppListFetch =
  | { state: 'started' }
  | ({ state: 'finished' } & SteamAppListFetch)
  | { state: 'failed'; reason: string };

/**
 * The order of the Library: Last Played (most recent first), title, or
 * Playtime (longest first). Games without a value come last, Games with the
 * same value by title. Mirrors `LibrarySort` in crates/core.
 */
export type LibrarySort = 'recentlyPlayed' | 'name' | 'playtime' | 'progress';

/**
 * What the Library is narrowed to, and in which order; every part that is set
 * must match. Mirrors `LibraryFilter` in crates/core.
 */
export interface LibraryFilter {
  /** Text the title must contain, ignoring case; empty matches every title. */
  search: string;
  /** Only Games of this Source; `null` for both Sources. */
  source: Source | null;
  /** Only Installed Games. */
  installedOnly: boolean;
  /** The order the Games are answered in. */
  sort: LibrarySort;
  /** Only Games with this Status; `null` for every Status. */
  status: Status | null;
  /** Only Favorites. */
  favoritesOnly: boolean;
  /** Only Games in the Collection with this id, the one chosen in the sidebar; left out for every Game. */
  collection?: number;
}

/**
 * A Session of a Local Game that began or ended, noticed by the core however
 * the Game was started. Mirrors `Tracking` in crates/core; a failed look at
 * the running processes never reaches the interface.
 */
export type SessionEvent = { state: 'started'; gameId: string } | { state: 'ended'; gameId: string };

/** One Achievement of a Steam Game, with whether the Steam Account unlocked it. Mirrors `Achievement` in crates/core. */
export interface Achievement {
  /** Steam's name for it, unique within the Game. */
  id: string;
  name: string;
  description: string | null;
  unlocked: boolean;
  /** Seconds since the Unix epoch; `null` while locked or when Steam does not know when. */
  unlockedAt: number | null;
  /** The icon as the Achievement looks now, for `achievementIconUrl`; `null` when Steam has none. */
  icon: string | null;
}

/** The Achievements of one Game. Mirrors `GameAchievements` in crates/core. */
export type GameAchievements =
  | { state: 'localGame' }
  | { state: 'notFetchedYet' }
  | { state: 'noAchievements' }
  | {
      state: 'listed';
      unlocked: number;
      total: number;
      /** Unlocked ones first, the most recently unlocked first, then the locked ones in Steam's order. */
      achievements: Achievement[];
    };
/** How the Library shows its Games: a grid of Covers or one row per Game. Mirrors `LibraryLayout` in crates/core. */
export type LibraryLayout = 'coverGrid' | 'list';

export interface Operations {
  /** All Games in the Library as last stored, ordered by title. Answers at once. */
  library(): Promise<Game[]>;
  /** Reads which Steam Games are Installed, stores it and answers the Library. */
  readInstalledGames(): Promise<Game[]>;
  /** The Steam installation and its Steam Accounts, the most recently used first. */
  steamInstallation(): Promise<SteamInstallation>;
  /**
   * The SteamID of the Steam Account the app works with: the one the user
   * chose, until then the most recently used one; `null` when none is known.
   */
  steamAccount(): Promise<string | null>;
  /**
   * Makes a SteamID the Steam Account the app works with, also after a
   * restart: one of the accounts on this computer or one entered by hand.
   * Answers `invalidSteamId`, and changes nothing, for anything that is not one.
   */
  chooseSteamAccount(steamId: string): Promise<SteamAccountChoice>;
  /**
   * The address an `<img>` loads the Game's Cover from. The core downloads
   * and caches it; when the Game has none, the image fails to load. A new
   * `revision` gives a new address, so the image loads again after the
   * Cover changed.
   */
  coverUrl(game: Game, revision?: number): string;
  /**
   * Launches an Installed Game. A Steam Game starts through Steam itself, a
   * Local Game by starting its executable with its Launch Options.
   * Rejects when the Game could not be launched.
   */
  launch(game: Game): Promise<LaunchOutcome>;
  /** Whether a Steam Web API key is stored. The key itself never reaches the interface. */
  steamWebApiKeyStored(): Promise<boolean>;
  /**
   * Checks a pasted Steam Web API key with Steam and stores it in the
   * operating system's secret store in place of any earlier one. A key that
   * is `invalid`, or that cannot be checked because Steam is unreachable,
   * changes nothing. Rejects when the key could not be stored. A stored key
   * starts an automatic Sync right away (`onAutomaticSync`).
   */
  storeSteamWebApiKey(key: string): Promise<SteamWebApiKeyCheck>;
  /** Removes the stored Steam Web API key, if there is one. */
  removeSteamWebApiKey(): Promise<void>;
  /** Opens the Steam page where the user creates their key, in the browser. */
  openSteamWebApiKeyPage(): Promise<void>;
  /**
   * Fetches every Steam Game the Steam Account owns, with Playtime and Last
   * Played, and merges it into the stored Library. Any outcome but `synced`,
   * and a rejection, leaves the stored data as it was.
   */
  sync(): Promise<SyncOutcome>;
  /** When the last successful Sync happened, in seconds since the Unix epoch; `null` before the first. */
  lastSync(): Promise<number | null>;
  /**
   * Hears the automatic Syncs the core runs on its own, shortly after the
   * start, right after a key is stored, and then every 30 minutes. Answers
   * the function that stops listening.
   */
  onAutomaticSync(listener: (event: AutomaticSync) => void): () => void;
  /**
   * Asks the user to choose a Local Game's executable in the operating
   * system's file dialog and proposes a title for it. Rejects when no dialog
   * can be shown.
   */
  chooseLocalGameExecutable(): Promise<ExecutableChoice>;
  /**
   * Adds the executable to the Library as a Local Game with the title the
   * user confirmed or corrected. The same executable is never added twice.
   */
  addLocalGame(executable: string, title: string): Promise<AddLocalGame>;
  /** The theme the user chose; `system` until they choose. */
  theme(): Promise<Theme>;
  /** Makes a theme the theme of the interface, also after a restart. */
  chooseTheme(theme: Theme): Promise<void>;
  /** Every Collection with how many Games of the Library are in it, ordered by name. */
  collections(): Promise<Collection[]>;
  /**
   * Creates a Collection; the name is saved without the spaces around it.
   * An empty name, or one another Collection has in any case, is refused
   * and creates nothing.
   */
  createCollection(name: string): Promise<CollectionNaming>;
  /** Renames a Collection under the same rules as `createCollection`. Rejects when there is no such Collection. */
  renameCollection(id: number, name: string): Promise<CollectionNaming>;
  /** Deletes a Collection. Its Games stay in the Library. */
  deleteCollection(id: number): Promise<void>;
  /** What is stored of the Steam App List, the basis for Cover Matches. */
  steamAppList(): Promise<SteamAppList>;
  /**
   * Fetches the Steam App List with the stored Steam Web API key, page by
   * page. An unfinished fetch is continued where it stopped; otherwise the
   * whole list is fetched anew while the stored one stays usable.
   */
  fetchSteamAppList(): Promise<SteamAppListFetch>;
  /**
   * Hears the fetches of the Steam App List the core runs on its own: a
   * minute after the start, right after a key is stored, and whenever the
   * list is a week old. Answers the function that stops listening.
   */
  onAutomaticSteamAppListFetch(listener: (event: AutomaticSteamAppListFetch) => void): () => void;
  /** The Games in the stored Library that match the filter, in the order it asks for. */
  filterLibrary(filter: LibraryFilter): Promise<Game[]>;
  /** The Game as the Library shows it, for its detail page; `null` when it is no longer in the Library. */
  game(gameId: string): Promise<Game | null>;
  /** The ids of the Collections the Game is in, ordered like `collections`; empty when it is in none. */
  gameCollections(gameId: string): Promise<number[]>;
  /**
   * Puts a Game into a Collection, besides the other Collections it is in.
   * Rejects when there is no such Collection or the Game is not in the Library.
   */
  addGameToCollection(collection: number, gameId: string): Promise<void>;
  /** Takes a Game out of a Collection; it stays in the Library and in its other Collections. */
  removeGameFromCollection(collection: number, gameId: string): Promise<void>;
  /** How the Library shows its Games; `coverGrid` until the user switches. */
  libraryLayout(): Promise<LibraryLayout>;
  /** Makes a Library Layout the one the Library shows, also after a restart. */
  chooseLibraryLayout(layout: LibraryLayout): Promise<void>;
  /** Whether closing the window keeps the app running in the Tray. On until the user turns it off. */
  closeToTray(): Promise<boolean>;
  /** Turns close-to-tray on or off, also after a restart. */
  setCloseToTray(on: boolean): Promise<void>;
  /**
   * Closes the window as its close button does: with close-to-tray on the
   * window hides and the app keeps running in the Tray, otherwise it quits.
   */
  closeWindow(): Promise<void>;
  /**
   * Hears the first close of the window with close-to-tray on. The window
   * then stays open to explain the Tray, and `closeWindow` closes it once
   * the user has read that. Answers the function that stops listening.
   */
  onTrayNotice(listener: () => void): () => void;
  /** Gives the Tray icon its tooltip and menu, in the interface language. */
  showTrayMenu(texts: TrayMenuTexts): Promise<void>;
  /** The order the user chose for the Library; `recentlyPlayed` until they choose. */
  librarySort(): Promise<LibrarySort>;
  /** Makes `sort` the order of the Library, also after a restart. */
  chooseLibrarySort(sort: LibrarySort): Promise<void>;
  /** The interface language the user chose; `system` until they choose. */
  language(): Promise<Language>;
  /** Makes `language` the interface language, also after a restart. */
  chooseLanguage(language: Language): Promise<void>;
  /**
   * Sets a Local Game's Manual Progress to a whole number from 0 to 100, or
   * clears it with `null`; it is then the Game's Progress. Anything outside
   * 0 to 100 is `outOfRange`, a Steam Game `notALocalGame`; both change nothing.
   */
  setManualProgress(gameId: string, percent: number | null): Promise<ManualProgress>;
  /**
   * Asks the user to choose an image in the operating system's file dialog
   * and makes it the Cover of a Local Game, in place of its executable's
   * icon. The image is copied into the app's data folder. Rejects for a Game
   * that is not a Local Game, and when no dialog can be shown.
   */
  chooseCoverImage(gameId: string): Promise<CoverImageChoice>;
  /** Whether the user chose an image as the Cover of a Local Game. */
  coverImageChosen(gameId: string): Promise<boolean>;
  /** Forgets the chosen image: the Local Game's Cover is its executable's icon again. */
  removeCoverImage(gameId: string): Promise<void>;
  /**
   * Sets the Status of a Game, also after a restart, and answers the Game as
   * the Library now shows it. Rejects for a Game that is not in the Library.
   */
  setStatus(gameId: string, status: Status): Promise<Game>;
  /**
   * Shows the folder of the log files in File Explorer, so the user can attach
   * one to a bug report themselves. The app never sends the log. Rejects when
   * the folder could not be shown.
   */
  openLogFolder(): Promise<void>;
  /**
   * Whether the app starts with Windows, hidden in the Tray: what the current
   * user's autostart entry says, also after it was changed outside the app.
   */
  startsWithWindows(): Promise<boolean>;
  /** Adds or removes the current user's autostart entry. */
  setStartWithWindows(on: boolean): Promise<void>;
  /**
   * Hears each Session of a Local Game begin and end while the app runs;
   * each one changes the Game's Playtime and Last Played. Answers the
   * function that stops listening.
   */
  onSession(listener: (event: SessionEvent) => void): () => void;
  /** The Achievements of the Game as the last Syncs fetched them. Answers from the store at once. */
  achievements(gameId: string): Promise<GameAchievements>;
  /**
   * The address an `<img>` loads an Achievement's icon from. The core
   * downloads and caches it like a Cover; when there is none, the image fails to load.
   */
  achievementIconUrl(game: Game, icon: string): string;
  /** How a Local Game is launched; `null` for anything but a Local Game in the Library. */
  launchOptions(gameId: string): Promise<LaunchOptions | null>;
  /**
   * Sets how a Local Game is launched, also after a restart. A Working
   * Directory of only spaces is none: the executable's folder is used.
   * Rejects for anything but a Local Game in the Library.
   */
  setLaunchOptions(gameId: string, options: LaunchOptions): Promise<void>;
  /** The Hidden Games, ordered by title like `library`. They stay hidden through Syncs and restarts. */
  hiddenGames(): Promise<Game[]>;
  /** Hides a Game: it leaves the Library and every view built on it without being deleted. */
  hideGame(gameId: string): Promise<void>;
  /** Brings a Hidden Game back into the Library. */
  unhideGame(gameId: string): Promise<void>;
  /** A Local Game with its title and executable, for editing; `null` for a Steam Game or one no longer in the Library. */
  localGame(gameId: string): Promise<LocalGame | null>;
  /**
   * Asks the user to choose a new executable for a Local Game in the file
   * dialog, as `chooseLocalGameExecutable` does; the Game's own executable
   * does not count as already added. Rejects for a Steam Game.
   */
  chooseExecutableFor(gameId: string): Promise<ExecutableChoice>;
  /**
   * Gives a Local Game a new title and executable; it keeps everything else.
   * Refused like `addLocalGame`. Rejects for a Steam Game or one that is gone.
   */
  editLocalGame(gameId: string, title: string, executable: string): Promise<EditLocalGame>;
  /**
   * Removes a Local Game from the Library with everything the app keeps
   * about it; its files on disk stay. Rejects for a Steam Game.
   */
  removeLocalGame(gameId: string): Promise<void>;
  /**
   * The Statistics of the Library as it shows, worked out by the core;
   * Hidden Games count in none of them.
   */
  statistics(): Promise<Statistics>;
  /**
   * Marks a Game as a Favorite, or with `false` unmarks it. The mark stays
   * through Syncs and restarts. Rejects when the Game is not in the Library.
   */
  markFavorite(gameId: string, favorite: boolean): Promise<void>;
  /** How many Games of the Library are Favorites. */
  favoriteCount(): Promise<number>;
  /** The version of the app that runs and its license, for the About dialog. */
  about(): Promise<About>;
  /** Opens the project page or the license in the browser. */
  openAboutPage(page: AboutPage): Promise<void>;
}

/** How choosing an image as a Local Game's Cover went. Every outcome but `chosen` changed nothing. Mirrors `CoverImageChoice` in crates/core. */
export type CoverImageChoice = { outcome: 'chosen' } | { outcome: 'cancelled' } | { outcome: 'notAnImage' };

/** What the About dialog tells of the app itself. Mirrors `About` in crates/core. */
export interface About {
  /** Such as `0.1.0`. */
  version: string;
  /** The SPDX identifier of the license, such as `MIT`. */
  license: string;
}

/** A page the About dialog links to. Mirrors `AboutPage` in crates/core. */
export type AboutPage = 'project' | 'license';

/**
 * The interface language the user chose; `system` follows the system language, English for anything
 * but German. Mirrors `Language` in crates/core.
 */
export type Language = 'system' | 'de' | 'en';

/** The texts of the Tray icon. Mirrors `TrayMenuTexts` in src-tauri/src/tray.rs. */
export interface TrayMenuTexts {
  tooltip: string;
  /** The menu entry that opens the window. */
  open: string;
  /** The menu entry that quits the app. */
  quit: string;
}
export const tauriOperations: Operations = {
  library: () => invoke<Game[]>('library'),
  readInstalledGames: () => invoke<Game[]>('read_installed_games'),
  steamInstallation: () => invoke<SteamInstallation>('steam_installation'),
  steamAccount: () => invoke<string | null>('steam_account'),
  chooseSteamAccount: (steamId) => invoke<SteamAccountChoice>('choose_steam_account', { steamId }),
  // The `cover` URL scheme of the desktop shell (COVER_PROTOCOL in src-tauri/src/lib.rs).
  // The shell reads only the path; the revision is for the interface's image cache.
  coverUrl: (game, revision) =>
    `${convertFileSrc(game.id, 'cover')}${revision ? `?revision=${revision}` : ''}`,
  launch: (game) => invoke<LaunchOutcome>('launch', { gameId: game.id }),
  steamWebApiKeyStored: () => invoke<boolean>('steam_web_api_key_stored'),
  storeSteamWebApiKey: (key) => invoke<SteamWebApiKeyCheck>('store_steam_web_api_key', { key }),
  removeSteamWebApiKey: () => invoke<void>('remove_steam_web_api_key'),
  openSteamWebApiKeyPage: () => invoke<void>('open_steam_web_api_key_page'),
  sync: () => invoke<SyncOutcome>('sync'),
  lastSync: () => invoke<number | null>('last_sync'),
  onAutomaticSync: (listener) => {
    // AUTOMATIC_SYNC_EVENT in src-tauri/src/lib.rs.
    const listening = listen<AutomaticSync>('automatic-sync', (event) => listener(event.payload));
    return () => {
      void listening.then((stop) => stop());
    };
  },
  chooseLocalGameExecutable: () => invoke<ExecutableChoice>('choose_local_game_executable'),
  addLocalGame: (executable, title) => invoke<AddLocalGame>('add_local_game', { executable, title }),
  theme: () => invoke<Theme>('theme'),
  chooseTheme: (theme) => invoke<void>('choose_theme', { theme }),
  collections: () => invoke<Collection[]>('collections'),
  createCollection: (name) => invoke<CollectionNaming>('create_collection', { name }),
  renameCollection: (id, name) => invoke<CollectionNaming>('rename_collection', { id, name }),
  deleteCollection: (id) => invoke<void>('delete_collection', { id }),
  steamAppList: () => invoke<SteamAppList>('steam_app_list'),
  fetchSteamAppList: () => invoke<SteamAppListFetch>('fetch_steam_app_list'),
  onAutomaticSteamAppListFetch: (listener) => {
    // AUTOMATIC_STEAM_APP_LIST_FETCH_EVENT in src-tauri/src/lib.rs.
    const listening = listen<AutomaticSteamAppListFetch>('automatic-steam-app-list-fetch', (event) =>
      listener(event.payload),
    );
    return () => {
      void listening.then((stop) => stop());
    };
  },
  filterLibrary: (filter) => invoke<Game[]>('filter_library', { filter }),
  game: (gameId) => invoke<Game | null>('game', { gameId }),
  gameCollections: (gameId) => invoke<number[]>('game_collections', { gameId }),
  addGameToCollection: (collection, gameId) => invoke<void>('add_game_to_collection', { collection, gameId }),
  removeGameFromCollection: (collection, gameId) => invoke<void>('remove_game_from_collection', { collection, gameId }),
  libraryLayout: () => invoke<LibraryLayout>('library_layout'),
  chooseLibraryLayout: (layout) => invoke<void>('choose_library_layout', { layout }),
  closeToTray: () => invoke<boolean>('close_to_tray'),
  setCloseToTray: (on) => invoke<void>('set_close_to_tray', { on }),
  closeWindow: () => invoke<void>('close_window'),
  onTrayNotice: (listener) => {
    // TRAY_NOTICE_EVENT in src-tauri/src/tray.rs.
    const listening = listen('tray-notice', () => listener());
    return () => {
      void listening.then((stop) => stop());
    };
  },
  showTrayMenu: (texts) => invoke<void>('show_tray_menu', { texts }),
  librarySort: () => invoke<LibrarySort>('library_sort'),
  chooseLibrarySort: (sort) => invoke<void>('choose_library_sort', { sort }),
  setManualProgress: (gameId, percent) => invoke<ManualProgress>('set_manual_progress', { gameId, percent }),
  chooseCoverImage: (gameId) => invoke<CoverImageChoice>('choose_cover_image', { gameId }),
  coverImageChosen: (gameId) => invoke<boolean>('cover_image_chosen', { gameId }),
  removeCoverImage: (gameId) => invoke<void>('remove_cover_image', { gameId }),
  language: () => invoke<Language>('language'),
  chooseLanguage: (language) => invoke<void>('choose_language', { language }),
  setStatus: (gameId, status) => invoke<Game>('set_status', { gameId, status }),
  openLogFolder: () => invoke<void>('open_log_folder'),
  startsWithWindows: () => invoke<boolean>('starts_with_windows'),
  setStartWithWindows: (on) => invoke<void>('set_start_with_windows', { on }),
  onSession: (listener) => {
    // SESSION_EVENT in src-tauri/src/lib.rs.
    const listening = listen<SessionEvent>('session', (event) => listener(event.payload));
    return () => {
      void listening.then((stop) => stop());
    };
  },
  achievements: (gameId) => invoke<GameAchievements>('achievements', { gameId }),
  // Under the Game's address in the `cover` URL scheme (`cover_response` in src-tauri/src/lib.rs).
  achievementIconUrl: (game, icon) => convertFileSrc(`${game.id}/achievements/${icon}`, 'cover'),
  launchOptions: (gameId) => invoke<LaunchOptions | null>('launch_options', { gameId }),
  setLaunchOptions: (gameId, options) => invoke<void>('set_launch_options', { gameId, options }),
  hiddenGames: () => invoke<Game[]>('hidden_games'),
  hideGame: (gameId) => invoke<void>('hide_game', { gameId }),
  unhideGame: (gameId) => invoke<void>('unhide_game', { gameId }),
  localGame: (gameId) => invoke<LocalGame | null>('local_game', { gameId }),
  chooseExecutableFor: (gameId) => invoke<ExecutableChoice>('choose_executable_for', { gameId }),
  editLocalGame: (gameId, title, executable) =>
    invoke<EditLocalGame>('edit_local_game', { gameId, title, executable }),
  removeLocalGame: (gameId) => invoke<void>('remove_local_game', { gameId }),
  statistics: () => invoke<Statistics>('statistics'),
  markFavorite: (gameId, favorite) => invoke<void>('mark_favorite', { gameId, favorite }),
  favoriteCount: () => invoke<number>('favorite_count'),
  about: () => invoke<About>('about'),
  openAboutPage: (page) => invoke<void>('open_about_page', { page }),
};

/** Mirrors `Statistics` in crates/core: the totals over the Games in the Library. */
export interface Statistics {
  /** How many Games the Library holds. */
  games: number;
  /** The Playtime of every Game together, in minutes. */
  playtimeMinutes: number;
  /** How many Achievements of all Steam Games are unlocked. */
  achievementsUnlocked: number;
  /** How many Achievements the Steam Games have together. */
  achievementsTotal: number;
  /** The share of unlocked Achievements in whole percent, rounded down; `null` while there are none. */
  achievementShare: number | null;
  /** How many Games are at 100 % Progress. */
  completeGames: number;
  /** How many Games have each Status. */
  gamesByStatus: Record<Status, number>;
  /** The Playtime of each of the last eight weeks, oldest first, from the recorded Sessions. */
  weeks: PlayedWeek[];
  /** The five Games with the most Playtime, most played first; a Game not played at all is none of them. */
  mostPlayed: Game[];
}

/** Mirrors `PlayedWeek` in crates/core: the Playtime of the seven days up to `endsAt`. */
export interface PlayedWeek {
  /** Where the week starts, in seconds since the Unix epoch. */
  startsAt: number;
  /** Where the week ends, in seconds since the Unix epoch; the latest week ends now. */
  endsAt: number;
  /** How long Sessions ran within the week, in whole minutes. */
  minutes: number;
}
