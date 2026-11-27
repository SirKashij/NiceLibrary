import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, InstallOutcome, Operations } from './operations';

function steamGame(appId: number, title: string, installed: boolean): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed, playtimeMinutes: null, lastPlayed: null, status: 'unplayed', progress: null, favorite: false };
}

/** Fake operations over a stored Library; `install` records each Game it is asked to install. */
function fakeOperations(games: Game[], install: (game: Game) => Promise<InstallOutcome>) {
  const installs: string[] = [];
  const launches: string[] = [];
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    exportData: async () => ({ outcome: 'cancelled' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    openLogFolder: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    filterLibrary: async () => [],
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    onAutomaticSync: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    launch: async (game) => {
      launches.push(game.id);
      return { outcome: 'launched' };
    },
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    onSession: () => () => {},
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    coverHue: async () => null,
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    language: async () => 'system',
    chooseLanguage: async () => {},
    install: (game) => {
      installs.push(game.id);
      return install(game);
    },
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
    importData: async () => ({ outcome: 'cancelled' }),
    limitedMode: async () => ({ outcome: 'off' }),
    openSteamPrivacySettings: async () => {},
    sessions: async () => [],
  };
  return { operations, installs, launches };
}

const started = async (): Promise<InstallOutcome> => ({ outcome: 'started' });

/** The drawing of a button's icon, to tell two icons apart. */
function iconOf(button: HTMLElement) {
  const icon = button.querySelector('svg');
  expect(icon).not.toBeNull();
  return icon!.innerHTML;
}

describe('Installing a Steam Game that is not Installed', () => {
  it('offers Install with its own label and icon where an Installed Game offers Play', async () => {
    const { operations } = fakeOperations([steamGame(400, 'Portal', false), steamGame(620, 'Portal 2', true)], started);
    render(<App operations={operations} lang="en" />);

    const [portal, portal2] = within(await screen.findByRole('list', { name: 'Games' })).getAllByRole('listitem');
    const install = within(portal).getByRole('button', { name: 'Install Portal' });
    const play = within(portal2).getByRole('button', { name: 'Play Portal 2' });
    expect(install).toHaveTextContent('Install');
    expect(play).toHaveTextContent('Play');
    expect(within(portal).queryByRole('button', { name: 'Play Portal' })).not.toBeInTheDocument();
    expect(within(portal2).queryByRole('button', { name: 'Install Portal 2' })).not.toBeInTheDocument();
    expect(iconOf(install)).not.toEqual(iconOf(play));
  });

  it('asks Steam to install the Game whose Install button is clicked', async () => {
    const user = userEvent.setup();
    const { operations, installs, launches } = fakeOperations(
      [steamGame(400, 'Portal', false), steamGame(620, 'Portal 2', false)],
      started,
    );
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Install Portal 2' }));

    expect(installs).toEqual(['steam:620']);
    expect(launches).toEqual([]);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('offers Install as the main action on the detail page of a Game that is not Installed', async () => {
    const user = userEvent.setup();
    const { operations, installs } = fakeOperations([steamGame(620, 'Portal 2', false)], started);
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Details for Portal 2' }));
    const detail = await screen.findByRole('region', { name: 'Portal 2' });
    expect(within(detail).queryByRole('button', { name: 'Play Portal 2' })).not.toBeInTheDocument();
    await user.click(within(detail).getByRole('button', { name: 'Install Portal 2' }));

    expect(installs).toEqual(['steam:620']);
  });

  it('says clearly when Steam is not available', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([steamGame(620, 'Portal 2', false)], async () => ({ outcome: 'steamNotFound' }));
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Install Portal 2' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Portal 2 could not be installed because Steam was not found on this computer.',
    );
  });

  it('says so when the installation could not be started', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([steamGame(620, 'Portal 2', false)], () =>
      Promise.reject(new Error('Windows could not open steam://install/620')),
    );
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Install Portal 2' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The installation of Portal 2 could not be started.');
  });

  it('speaks German', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([steamGame(620, 'Portal 2', false)], async () => ({ outcome: 'steamNotFound' }));
    render(<App operations={operations} lang="de" />);

    const install = await screen.findByRole('button', { name: 'Portal 2 installieren' });
    expect(install).toHaveTextContent('Installieren');
    await user.click(install);

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Portal 2 konnte nicht installiert werden, weil Steam auf diesem Computer nicht gefunden wurde.',
    );
  });
});
