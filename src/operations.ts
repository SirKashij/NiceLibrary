// The interface talks to the application core only through these operations.
// In the app they are Tauri commands; tests hand the interface a fake.
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type Source = 'steam' | 'local';

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
}

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

/** How a Launch went, when nothing failed. Mirrors `Launch` in crates/core. */
export type LaunchOutcome = { outcome: 'launched' } | { outcome: 'steamNotFound' };

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

/** What the Library is narrowed to; every part that is set must match. Mirrors `LibraryFilter` in crates/core. */
export interface LibraryFilter {
  /** Text the title must contain, ignoring case; empty matches every title. */
  search: string;
  /** Only Games of this Source; `null` for both Sources. */
  source: Source | null;
  /** Only Installed Games. */
  installedOnly: boolean;
}

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
   * and caches it; when the Game has none, the image fails to load.
   */
  coverUrl(game: Game): string;
  /**
   * Launches an Installed Game. A Steam Game starts through Steam itself.
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
  /** The Games in the stored Library that match the filter, ordered by title like `library`. */
  filterLibrary(filter: LibraryFilter): Promise<Game[]>;
  /** The Game as the Library shows it, for its detail page; `null` when it is no longer in the Library. */
  game(gameId: string): Promise<Game | null>;
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
}

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
  coverUrl: (game) => convertFileSrc(game.id, 'cover'),
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
  filterLibrary: (filter) => invoke<Game[]>('filter_library', { filter }),
  game: (gameId) => invoke<Game | null>('game', { gameId }),
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
  localGame: (gameId) => invoke<LocalGame | null>('local_game', { gameId }),
  chooseExecutableFor: (gameId) => invoke<ExecutableChoice>('choose_executable_for', { gameId }),
  editLocalGame: (gameId, title, executable) =>
    invoke<EditLocalGame>('edit_local_game', { gameId, title, executable }),
  removeLocalGame: (gameId) => invoke<void>('remove_local_game', { gameId }),
};
