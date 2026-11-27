import { act, render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { AutomaticSync, Game, Operations, SessionEvent } from './operations';

/** 2026-11-27 10:45:00 UTC, in seconds since the Unix epoch. */
const THIS_MORNING = 1_795_776_300;
/** 2026-11-20 20:15:00 UTC. */
const LAST_WEEK = 1_795_205_700;

const PORTAL_2: Game = {
  id: 'steam:620',
  title: 'Portal 2',
  source: 'steam',
  installed: true,
  playtimeMinutes: 90,
  lastPlayed: LAST_WEEK,
  status: 'playing',
  favorite: false,
  progress: null,
};

/**
 * Fake operations over a stored Library the test can change, a tracker whose
 * Session events the test sends, and automatic Syncs the test runs.
 */
function fakeOperations(initial: Game[]) {
  let games = initial;
  const sessionListeners = new Set<(event: SessionEvent) => void>();
  const syncListeners = new Set<(event: AutomaticSync) => void>();
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => true,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => LAST_WEEK,
    onAutomaticSync: (listener) => {
      syncListeners.add(listener);
      return () => syncListeners.delete(listener);
    },
    filterLibrary: async () => games,
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    onSession: (listener) => {
      sessionListeners.add(listener);
      return () => sessionListeners.delete(listener);
    },
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    language: async () => 'system',
    chooseLanguage: async () => {},
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    limitedMode: async () => ({ outcome: 'off' }),
    openSteamPrivacySettings: async () => {},
    sessions: async () => [],
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    coverHue: async () => null,
    install: async () => ({ outcome: 'started' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    exportData: async () => ({ outcome: 'cancelled' }),
    importData: async () => ({ outcome: 'cancelled' }),
  };
  const session = (event: SessionEvent) => act(() => sessionListeners.forEach((listener) => listener(event)));
  const sync = (event: AutomaticSync) => act(() => syncListeners.forEach((listener) => listener(event)));
  return { operations, session, sync, change: (next: Game[]) => (games = next) };
}

describe('Sessions of Steam Games', () => {
  it('shows the Playtime the Sync brings once a Session ends, Steam’s figure', async () => {
    const { operations, session, sync, change } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="en" />);
    const grid = await screen.findByRole('list', { name: 'Games' });
    expect(within(grid).getByText('1.5 h played · last Nov 20, 2026')).toBeInTheDocument();

    await session({ state: 'started', gameId: PORTAL_2.id });
    await session({ state: 'ended', gameId: PORTAL_2.id });
    await sync({ state: 'started' });
    expect(await screen.findByText('Syncing…')).toBeInTheDocument();
    change([{ ...PORTAL_2, playtimeMinutes: 150, lastPlayed: THIS_MORNING }]);
    await sync({ state: 'finished', outcome: 'synced', at: THIS_MORNING });

    expect(await within(grid).findByText('2.5 h played · last Nov 27, 2026')).toBeInTheDocument();
  });

  it('keeps showing Steam’s Playtime while a Session runs and after it ends without a Sync', async () => {
    const { operations, session } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="de" />);
    const grid = await screen.findByRole('list', { name: 'Spiele' });

    await session({ state: 'started', gameId: PORTAL_2.id });
    await session({ state: 'ended', gameId: PORTAL_2.id });

    expect(await within(grid).findByText(/1,5 Std\..*20\.11\.2026/)).toBeInTheDocument();
  });
});
