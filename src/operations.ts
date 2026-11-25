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
  // The `cover` URL scheme of the desktop shell (COVER_PROTOCOL in src-tauri/src/lib.rs).
  coverUrl: (game) => convertFileSrc(game.id, 'cover'),
  launch: (game) => invoke<LaunchOutcome>('launch', { gameId: game.id }),
};
