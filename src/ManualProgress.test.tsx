import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, ManualProgress, Operations } from './operations';

function localGame(id: number, title: string, progress: number | null = null): Game {
  return { id: `local:${id}`, title, source: 'local', installed: true, playtimeMinutes: 0, lastPlayed: null, progress, status: 'unplayed', favorite: false };
}

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
 * Fake operations over a stored Library whose Local Games take Manual
 * Progress from 0 to 100, as the core does, unless `setManualProgress` is
 * given. Calls are recorded.
 */
function fakeOperations(initial: Game[], options: { setManualProgress?: Operations['setManualProgress'] } = {}) {
  let games = initial;
  const calls = { setManualProgress: [] as [string, number | null][] };
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    exportData: async () => ({ outcome: 'cancelled' }),
    language: async () => 'system',
    chooseLanguage: async () => {},
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    onAutomaticSync: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    filterLibrary: async () => games,
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    onSession: () => () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    setManualProgress: (gameId, percent) => {
      calls.setManualProgress.push([gameId, percent]);
      if (options.setManualProgress) return options.setManualProgress(gameId, percent);
      const game = games.find((candidate) => candidate.id === gameId);
      if (!game || game.source !== 'local') return Promise.resolve<ManualProgress>({ outcome: 'notALocalGame' });
      if (percent !== null && (percent < 0 || percent > 100)) return Promise.resolve<ManualProgress>({ outcome: 'outOfRange' });
      const changed = { ...game, progress: percent };
      games = games.map((candidate) => (candidate.id === gameId ? changed : candidate));
      return Promise.resolve<ManualProgress>({ outcome: 'set', game: changed });
    },
  };
  return { operations, calls };
}

async function open(title: string, label = `Details for ${title}`) {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: label }));
  const page = await screen.findByRole('region', { name: title });
  return { user, page };
}

describe('Manual Progress of a Local Game', () => {
  it('shows no Progress while the Local Game has no Manual Progress', async () => {
    const { operations } = fakeOperations([localGame(1, 'Celeste')]);
    render(<App operations={operations} lang="en" />);

    const { page } = await open('Celeste');

    expect(within(page).getByLabelText('About Celeste')).not.toHaveTextContent('Progress');
    expect(within(page).getByRole('textbox', { name: 'Manual Progress in percent' })).toHaveValue('');
  });

  it('shows the Manual Progress as the Progress of the Game', async () => {
    const { operations } = fakeOperations([localGame(1, 'Celeste', 42)]);
    render(<App operations={operations} lang="en" />);

    const { page } = await open('Celeste');

    expect(within(page).getByLabelText('About Celeste')).toHaveTextContent('Progress42%');
    expect(within(page).getByRole('textbox', { name: 'Manual Progress in percent' })).toHaveValue('42');
  });

  it('sets the Manual Progress the user enters, and the Progress shows it', async () => {
    const { operations, calls } = fakeOperations([localGame(1, 'Celeste')]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.type(within(page).getByRole('textbox', { name: 'Manual Progress in percent' }), '75');
    await user.click(within(page).getByRole('button', { name: 'Save' }));

    expect(await within(page).findByLabelText('About Celeste')).toHaveTextContent('Progress75%');
    expect(calls.setManualProgress).toEqual([['local:1', 75]]);
  });

  it('accepts 0 and 100', async () => {
    const { operations, calls } = fakeOperations([localGame(1, 'Celeste')]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');
    const field = within(page).getByRole('textbox', { name: 'Manual Progress in percent' });

    await user.type(field, '0');
    await user.click(within(page).getByRole('button', { name: 'Save' }));
    expect(await within(page).findByLabelText('About Celeste')).toHaveTextContent('Progress0%');
    await user.clear(within(page).getByRole('textbox', { name: 'Manual Progress in percent' }));
    await user.type(within(page).getByRole('textbox', { name: 'Manual Progress in percent' }), '100');
    await user.click(within(page).getByRole('button', { name: 'Save' }));

    expect(await within(page).findByText('100%')).toBeInTheDocument();
    expect(calls.setManualProgress).toEqual([
      ['local:1', 0],
      ['local:1', 100],
    ]);
  });

  it('clears the Manual Progress, and the Game shows no Progress again', async () => {
    const { operations, calls } = fakeOperations([localGame(1, 'Celeste', 42)]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Clear' }));

    await expect.poll(() => within(page).getByLabelText('About Celeste').textContent).not.toContain('Progress');
    expect(within(page).getByRole('textbox', { name: 'Manual Progress in percent' })).toHaveValue('');
    expect(within(page).queryByRole('button', { name: 'Clear' })).not.toBeInTheDocument();
    expect(calls.setManualProgress).toEqual([['local:1', null]]);
  });

  it('clears the Manual Progress when the user saves an empty field', async () => {
    const { operations, calls } = fakeOperations([localGame(1, 'Celeste', 42)]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.clear(within(page).getByRole('textbox', { name: 'Manual Progress in percent' }));
    await user.click(within(page).getByRole('button', { name: 'Save' }));

    await expect.poll(() => calls.setManualProgress).toEqual([['local:1', null]]);
  });

  it('refuses a value above 100 as the core does, and keeps the Progress', async () => {
    const { operations, calls } = fakeOperations([localGame(1, 'Celeste', 42)]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');
    const field = within(page).getByRole('textbox', { name: 'Manual Progress in percent' });

    await user.clear(field);
    await user.type(field, '101');
    await user.click(within(page).getByRole('button', { name: 'Save' }));

    expect(await within(page).findByRole('alert')).toHaveTextContent('Enter a whole number from 0 to 100.');
    expect(within(page).getByLabelText('About Celeste')).toHaveTextContent('Progress42%');
    expect(calls.setManualProgress).toEqual([['local:1', 101]]);
  });

  it('refuses what is not a whole number without asking the core', async () => {
    const { operations, calls } = fakeOperations([localGame(1, 'Celeste')]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');
    const field = within(page).getByRole('textbox', { name: 'Manual Progress in percent' });

    for (const typed of ['12.5', 'half', '-3']) {
      await user.clear(field);
      await user.type(field, typed);
      await user.click(within(page).getByRole('button', { name: 'Save' }));
      expect(await within(page).findByRole('alert')).toHaveTextContent('Enter a whole number from 0 to 100.');
    }
    expect(calls.setManualProgress).toEqual([]);
  });

  it('says so when the Manual Progress could not be saved', async () => {
    const { operations } = fakeOperations([localGame(1, 'Celeste')], {
      setManualProgress: () => Promise.reject(new Error('database: locked')),
    });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.type(within(page).getByRole('textbox', { name: 'Manual Progress in percent' }), '50');
    await user.click(within(page).getByRole('button', { name: 'Save' }));

    expect(await within(page).findByRole('alert')).toHaveTextContent('The Manual Progress could not be saved.');
  });

  it('is in German for a German interface', async () => {
    const { operations } = fakeOperations([localGame(1, 'Celeste', 42)]);
    render(<App operations={operations} lang="de" />);

    const { user, page } = await open('Celeste', 'Details zu Celeste');

    expect(within(page).getByLabelText('Über Celeste')).toHaveTextContent('Fortschritt42 %');
    expect(within(page).getByRole('heading', { name: 'Manueller Fortschritt' })).toBeInTheDocument();
    const field = within(page).getByRole('textbox', { name: 'Manueller Fortschritt in Prozent' });
    expect(within(page).getByRole('button', { name: 'Entfernen' })).toBeInTheDocument();
    await user.clear(field);
    await user.type(field, '200');
    await user.click(within(page).getByRole('button', { name: 'Speichern' }));
    expect(await within(page).findByRole('alert')).toHaveTextContent('Gib eine ganze Zahl von 0 bis 100 ein.');
  });
});

describe('Steam Games', () => {
  it('offer no Manual Progress and show no Progress without one', async () => {
    const { operations } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="en" />);

    const { page } = await open('Portal 2');

    expect(within(page).queryByRole('textbox', { name: 'Manual Progress in percent' })).not.toBeInTheDocument();
    expect(within(page).queryByRole('heading', { name: 'Manual Progress' })).not.toBeInTheDocument();
    expect(within(page).getByLabelText('About Portal 2')).not.toHaveTextContent('Progress');
  });

  it('show the Progress their Source gives them', async () => {
    const { operations } = fakeOperations([{ ...PORTAL_2, progress: 60 }]);
    render(<App operations={operations} lang="en" />);

    const { page } = await open('Portal 2');

    expect(within(page).getByLabelText('About Portal 2')).toHaveTextContent('Progress60%');
  });
});
