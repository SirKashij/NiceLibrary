import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LaunchOptions, LaunchOutcome, Operations } from './operations';

const CELESTE: Game = {
  id: 'local:1',
  title: 'Celeste',
  source: 'local',
  installed: true,
  playtimeMinutes: 0,
  lastPlayed: null,
  progress: null,
  status: 'unplayed',
  favorite: false,
};

const PORTAL_2: Game = {
  id: 'steam:620',
  title: 'Portal 2',
  source: 'steam',
  installed: true,
  playtimeMinutes: null,
  lastPlayed: null,
  progress: null,
  status: 'unplayed',
  favorite: false,
};

/**
 * Fake operations over a Library of Celeste, a Local Game, and Portal 2, a
 * Steam Game. `launch` answers each Launch; the Launch Options of Celeste
 * are kept as the core would, and `saved` records each save.
 */
function fakeOperations({
  launch = async () => ({ outcome: 'launched' }),
  options = { arguments: '', workingDirectory: null },
  save = async () => {},
  load,
}: {
  launch?: (game: Game) => Promise<LaunchOutcome>;
  options?: LaunchOptions;
  save?: (options: LaunchOptions) => Promise<void>;
  load?: () => Promise<LaunchOptions | null>;
} = {}) {
  const games = [CELESTE, PORTAL_2];
  let stored = options;
  const saved: LaunchOptions[] = [];
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    launch,
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    onAutomaticSync: () => () => {},
    theme: async () => 'system',
    chooseTheme: async () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    filterLibrary: async () => games,
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    onSession: () => () => {},
    achievements: async () => ({ state: 'localGame' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launchOptions: load ?? (async (gameId) => (gameId === CELESTE.id ? stored : null)),
    setLaunchOptions: async (_gameId, next) => {
      saved.push(next);
      await save(next);
      stored = next;
    },
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
    language: async () => 'system',
    chooseLanguage: async () => {},
  };
  return { operations, saved };
}

async function openDetail(title: string) {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: new RegExp(`(Details for|Details zu) ${title}`) }));
  return user;
}

describe('Launching a Local Game', () => {
  it('says clearly when the executable is missing', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ launch: async () => ({ outcome: 'executableNotFound' }) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Play Celeste' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Celeste could not be launched because its executable was not found. It may have been moved or deleted.',
    );
  });

  it('says clearly in German when the executable is missing', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ launch: async () => ({ outcome: 'executableNotFound' }) });
    render(<App operations={operations} lang="de" />);

    await user.click(await screen.findByRole('button', { name: 'Celeste spielen' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Celeste konnte nicht gestartet werden, weil seine ausführbare Datei nicht gefunden wurde.',
    );
  });

  it('says clearly when the Working Directory is missing', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ launch: async () => ({ outcome: 'workingDirectoryNotFound' }) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Play Celeste' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Celeste could not be launched because its working directory was not found. Check its launch options.',
    );
  });

  it('says it on the Detail Page when the Game is launched from there', async () => {
    const { operations } = fakeOperations({ launch: async () => ({ outcome: 'executableNotFound' }) });
    render(<App operations={operations} lang="en" />);
    const user = await openDetail('Celeste');

    await user.click(screen.getByRole('button', { name: 'Play Celeste' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Celeste could not be launched');
  });
});

describe('The Launch Options of a Local Game', () => {
  it('shows the stored Launch Arguments and Working Directory on its Detail Page', async () => {
    const { operations } = fakeOperations({ options: { arguments: '-fast', workingDirectory: 'D:\\Saves' } });
    render(<App operations={operations} lang="en" />);
    await openDetail('Celeste');

    const form = await screen.findByRole('form', { name: 'Launch options' });
    expect(within(form).getByRole('textbox', { name: 'Launch arguments' })).toHaveValue('-fast');
    expect(within(form).getByRole('textbox', { name: 'Working directory' })).toHaveValue('D:\\Saves');
  });

  it('saves what the user typed, an empty Working Directory as none', async () => {
    const { operations, saved } = fakeOperations({ options: { arguments: '', workingDirectory: 'D:\\Old' } });
    render(<App operations={operations} lang="en" />);
    const user = await openDetail('Celeste');
    const form = await screen.findByRole('form', { name: 'Launch options' });

    await user.type(within(form).getByRole('textbox', { name: 'Launch arguments' }), '-iwad "D:\\My WADs\\doom2.wad"');
    await user.clear(within(form).getByRole('textbox', { name: 'Working directory' }));
    await user.click(within(form).getByRole('button', { name: 'Save' }));

    expect(saved).toEqual([{ arguments: '-iwad "D:\\My WADs\\doom2.wad"', workingDirectory: null }]);
    expect(await screen.findByRole('status')).toHaveTextContent('The launch options were saved.');
  });

  it('explains how arguments are separated and which folder is used without a Working Directory', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await openDetail('Celeste');
    const form = await screen.findByRole('form', { name: 'Launch options' });

    expect(within(form).getByRole('textbox', { name: 'Launch arguments' })).toHaveAccessibleDescription(
      'Separate arguments with spaces. Put an argument that contains spaces in double quotes.',
    );
    expect(within(form).getByRole('textbox', { name: 'Working directory' })).toHaveAccessibleDescription(
      'Leave it empty to start the game in the folder of its executable.',
    );
  });

  it('is in German in the German interface', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);
    await openDetail('Celeste');

    const form = await screen.findByRole('form', { name: 'Startoptionen' });
    expect(within(form).getByRole('textbox', { name: 'Startparameter' })).toBeInTheDocument();
    expect(within(form).getByRole('textbox', { name: 'Arbeitsverzeichnis' })).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Speichern' })).toBeInTheDocument();
  });

  it('says so when they could not be saved', async () => {
    const { operations } = fakeOperations({ save: () => Promise.reject(new Error('database is locked')) });
    render(<App operations={operations} lang="en" />);
    const user = await openDetail('Celeste');
    const form = await screen.findByRole('form', { name: 'Launch options' });

    await user.type(within(form).getByRole('textbox', { name: 'Launch arguments' }), '-fast');
    await user.click(within(form).getByRole('button', { name: 'Save' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The launch options could not be saved.');
  });

  it('says so when they could not be loaded', async () => {
    const { operations } = fakeOperations({ load: () => Promise.reject(new Error('database is locked')) });
    render(<App operations={operations} lang="en" />);
    await openDetail('Celeste');

    expect(await screen.findByRole('alert')).toHaveTextContent('The launch options could not be loaded.');
    expect(screen.queryByRole('form', { name: 'Launch options' })).not.toBeInTheDocument();
  });

  it('are not offered for a Steam Game', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await openDetail('Portal 2');

    await screen.findByRole('heading', { level: 1, name: 'Portal 2' });
    expect(screen.queryByRole('form', { name: 'Launch options' })).not.toBeInTheDocument();
  });
});
