import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, Operations } from './operations';

function steamGame(appId: number, title: string): Game {
  return {
    id: `steam:${appId}`,
    title,
    source: 'steam',
    installed: true,
    playtimeMinutes: null,
    lastPlayed: null,
    progress: null,
    status: 'unplayed',
    favorite: false,
  };
}

const HADES = steamGame(1145360, 'Hades');
const PORTAL_2 = steamGame(620, 'Portal 2');

/**
 * Fake operations that hide and unhide like the core: a Hidden Game leaves
 * the Library and its search results and is listed among the Hidden Games.
 */
function fakeOperations(games: Game[], options: { hidden?: string[]; hideFails?: boolean } = {}) {
  const hidden = new Set(options.hidden ?? []);
  const shown = () => games.filter((game) => !hidden.has(game.id));
  const operations: Operations = {
    library: async () => shown(),
    readInstalledGames: async () => shown(),
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    onAutomaticSync: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    hiddenGames: async () => games.filter((game) => hidden.has(game.id)),
    hideGame: async (gameId) => {
      if (options.hideFails) throw new Error('database: locked');
      hidden.add(gameId);
    },
    unhideGame: async (gameId) => {
      hidden.delete(gameId);
    },
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    filterLibrary: async () => shown(),
    game: async (gameId) => shown().find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    onSession: () => () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
  };
  return { operations, hidden };
}

async function libraryTitles() {
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  return within(grid)
    .getAllByRole('heading')
    .map((heading) => heading.textContent);
}

describe('Hidden Games', () => {
  it('hides a Game from its detail page, so it leaves the Library', async () => {
    const user = userEvent.setup();
    const { operations, hidden } = fakeOperations([HADES, PORTAL_2]);
    render(<App operations={operations} lang="en" />);

    const grid = await screen.findByRole('list', { name: 'Games' });
    await user.click(within(grid).getByRole('button', { name: 'Details for Portal 2' }));
    await user.click(await screen.findByRole('button', { name: 'Hide Portal 2' }));

    expect(hidden.has(PORTAL_2.id)).toBe(true);
    expect(await screen.findByRole('status')).toHaveTextContent('Portal 2 is hidden. You find it under Hidden games.');
    expect(screen.getByRole('heading', { level: 1, name: 'Library' })).toBeVisible();
    await expect.poll(libraryTitles).toEqual(['Hades']);
  });

  it('stays on the detail page and says so when a Game cannot be hidden', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([HADES, PORTAL_2], { hideFails: true });
    render(<App operations={operations} lang="en" />);

    const grid = await screen.findByRole('list', { name: 'Games' });
    await user.click(within(grid).getByRole('button', { name: 'Details for Hades' }));
    await user.click(await screen.findByRole('button', { name: 'Hide Hades' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Hades could not be hidden.');
    expect(screen.getByRole('heading', { level: 1, name: 'Hades' })).toBeVisible();
  });

  it('lists the Hidden Games and unhides one back into the Library', async () => {
    const user = userEvent.setup();
    const { operations, hidden } = fakeOperations([HADES, PORTAL_2], { hidden: [PORTAL_2.id] });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Hidden games' }));
    const list = await screen.findByRole('list', { name: 'Hidden games' });
    expect(within(list).getByText('Portal 2')).toBeVisible();
    await user.click(within(list).getByRole('button', { name: 'Unhide Portal 2' }));

    expect(hidden.has(PORTAL_2.id)).toBe(false);
    expect(await screen.findByText('No game is hidden.')).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Back to the library' }));
    await expect.poll(libraryTitles).toEqual(['Hades', 'Portal 2']);
  });

  it('says when no Game is hidden', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([HADES]);
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Hidden games' }));

    expect(await screen.findByRole('heading', { level: 1, name: 'Hidden games' })).toBeVisible();
    expect(await screen.findByText('No game is hidden.')).toBeVisible();
  });

  it('speaks German', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([HADES, PORTAL_2], { hidden: [PORTAL_2.id] });
    render(<App operations={operations} lang="de" />);

    const grid = await screen.findByRole('list', { name: 'Spiele' });
    await user.click(within(grid).getByRole('button', { name: 'Details zu Hades' }));
    expect(await screen.findByRole('button', { name: 'Hades ausblenden' })).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Zurück zur Bibliothek' }));

    await user.click(screen.getByRole('button', { name: 'Ausgeblendete Spiele' }));
    const list = await screen.findByRole('list', { name: 'Ausgeblendete Spiele' });
    expect(within(list).getByRole('button', { name: 'Portal 2 einblenden' })).toBeVisible();
  });
});
