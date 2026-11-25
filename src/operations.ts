// The interface talks to the application core only through these operations.
// In the app they are Tauri commands; tests hand the interface a fake.
import { convertFileSrc, invoke } from '@tauri-apps/api/core';

export type Source = 'steam' | 'local';

/** One playable title in the Library. Mirrors `Game` in crates/core. */
export interface Game {
  /** Unique across Sources: `steam:<app id>` for a Steam Game. */
  id: string;
  title: string;
  source: Source;
  installed: boolean;
}

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
};
