import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LibraryFilter, LibrarySort, Operations } from './operations';

function steamGame(appId: number, title: string, playtimeMinutes: number | null, lastPlayed: number | null): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed: true, playtimeMinutes, lastPlayed, status: 'unplayed', favorite: false };
}

// Ordered by title, as `library` answers.
const LIBRARY = [
  steamGame(1145360, 'Hades', 90, 1_750_000_000),
  steamGame(220, 'Half-Life 2', null, null),
  steamGame(620, 'Portal 2', 600, 1_700_000_000),
];

/** How the core orders LIBRARY for each order. */
const ORDERED: Record<LibrarySort, string[]> = {
  recentlyPlayed: ['Hades', 'Portal 2', 'Half-Life 2'],
  name: ['Hades', 'Half-Life 2', 'Portal 2'],
  playtime: ['Portal 2', 'Hades', 'Half-Life 2'],
};

/**
 * Fake operations over LIBRARY with `stored` as the chosen order. They record
 * each filter asked for and each order chosen.
 */
function fakeOperations(stored: LibrarySort = 'recentlyPlayed') {
  const asked: LibraryFilter[] = [];
  const chosen: LibrarySort[] = [];
  const operations: Operations = {
    library: async () => LIBRARY,
    readInstalledGames: async () => LIBRARY,
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
    onAutomaticSync: () => () => {},
    filterLibrary: async (filter) => {
      asked.push(filter);
      return ORDERED[filter.sort]
        .map((title) => LIBRARY.find((game) => game.title === title)!)
        .filter((game) => game.title.toLowerCase().includes(filter.search.toLowerCase()));
    },
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    game: async (gameId) => LIBRARY.find((game) => game.id === gameId) ?? null,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    theme: async () => 'system',
    language: async () => 'system',
    chooseLanguage: async () => {},
    chooseTheme: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    openLogFolder: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    onSession: () => () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    librarySort: async () => stored,
    chooseLibrarySort: async (sort) => {
      chosen.push(sort);
      stored = sort;
    },
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
  };
  return { operations, asked, chosen };
}

/** The titles the Library shows, in the order it shows them. */
function shownTitles() {
  const list = screen.queryByRole('list', { name: 'Games' });
  return list ? within(list).queryAllByRole('heading').map((heading) => heading.textContent) : [];
}

describe('Sorting the Library', () => {
  it('shows the Library recently played first until the user chooses another order', async () => {
    const { operations, asked } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await waitFor(() => expect(shownTitles()).toEqual(['Hades', 'Portal 2', 'Half-Life 2']));
    expect(screen.getByRole('combobox', { name: 'Sort by' })).toHaveValue('recentlyPlayed');
    expect(asked.at(-1)).toEqual({ search: '', source: null, installedOnly: false, status: null, sort: 'recentlyPlayed', favoritesOnly: false });
  });

  it('offers recently played, name and Playtime', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    const sort = await screen.findByRole('combobox', { name: 'Sort by' });
    expect(within(sort).getAllByRole('option').map((option) => option.textContent)).toEqual([
      'Recently played',
      'Name',
      'Playtime',
    ]);
  });

  it('asks the core for the chosen order and keeps it for the next start', async () => {
    const user = userEvent.setup();
    const { operations, asked, chosen } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await waitFor(() => expect(shownTitles()).toEqual(ORDERED.recentlyPlayed));

    await user.selectOptions(screen.getByRole('combobox', { name: 'Sort by' }), 'Playtime');

    await waitFor(() => expect(shownTitles()).toEqual(['Portal 2', 'Hades', 'Half-Life 2']));
    expect(asked.at(-1)?.sort).toBe('playtime');
    expect(chosen).toEqual(['playtime']);
  });

  it('starts with the order chosen before', async () => {
    const { operations } = fakeOperations('playtime');
    render(<App operations={operations} lang="en" />);

    await waitFor(() => expect(shownTitles()).toEqual(['Portal 2', 'Hades', 'Half-Life 2']));
    expect(screen.getByRole('combobox', { name: 'Sort by' })).toHaveValue('playtime');
  });

  it('sorts what the search leaves, and keeps the order when the filters are reset', async () => {
    const user = userEvent.setup();
    const { operations, asked } = fakeOperations('playtime');
    render(<App operations={operations} lang="en" />);
    await waitFor(() => expect(shownTitles()).toEqual(ORDERED.playtime));

    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'ha');
    await waitFor(() => expect(shownTitles()).toEqual(['Hades', 'Half-Life 2']));
    expect(asked.at(-1)).toEqual({ search: 'ha', source: null, installedOnly: false, status: null, sort: 'playtime', favoritesOnly: false });

    await user.click(screen.getByRole('checkbox', { name: 'Installed only' }));
    await user.click(screen.getByRole('checkbox', { name: 'Installed only' }));
    await user.clear(screen.getByRole('searchbox', { name: 'Search the library' }));

    await waitFor(() => expect(shownTitles()).toEqual(ORDERED.playtime));
    expect(screen.getByRole('combobox', { name: 'Sort by' })).toHaveValue('playtime');
  });

  it('keeps the order for now and says so when it could not be kept for the next start', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    operations.chooseLibrarySort = () => Promise.reject(new Error('database: disk I/O error'));
    render(<App operations={operations} lang="en" />);
    await waitFor(() => expect(shownTitles()).toEqual(ORDERED.recentlyPlayed));

    await user.selectOptions(screen.getByRole('combobox', { name: 'Sort by' }), 'Name');

    await waitFor(() => expect(shownTitles()).toEqual(ORDERED.name));
    expect(await screen.findByRole('alert')).toHaveTextContent('The sort order could not be saved for the next start.');
  });

  it('sorts recently played first when the chosen order cannot be read', async () => {
    const { operations } = fakeOperations('playtime');
    operations.librarySort = () => Promise.reject(new Error('database: disk I/O error'));
    render(<App operations={operations} lang="en" />);

    await waitFor(() => expect(shownTitles()).toEqual(ORDERED.recentlyPlayed));
    expect(screen.getByRole('combobox', { name: 'Sort by' })).toHaveValue('recentlyPlayed');
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);

    const sort = await screen.findByRole('combobox', { name: 'Sortieren nach' });
    expect(within(sort).getAllByRole('option').map((option) => option.textContent)).toEqual([
      'Zuletzt gespielt',
      'Name',
      'Spielzeit',
    ]);
  });
});
