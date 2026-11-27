import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LibraryFilter, Operations } from './operations';

function steamGame(appId: number, title: string, installed: boolean): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed, playtimeMinutes: null, lastPlayed: null, progress: null, status: 'unplayed', favorite: false };
}

const LIBRARY = [steamGame(1145360, 'Hades', true), steamGame(220, 'Half-Life 2', false), steamGame(620, 'Portal 2', true)];

/**
 * Fake operations over `library`. `filterLibrary` records each filter it is
 * asked for and answers like the core does.
 */
function fakeOperations(library: Game[] = LIBRARY) {
  const asked: LibraryFilter[] = [];
  const operations: Operations = {
    library: async () => library,
    readInstalledGames: async () => library,
    steamInstallation: async () => ({ state: 'notFound' }),
    exportData: async () => ({ outcome: 'cancelled' }),
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
    openLogFolder: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    theme: async () => 'system',
    language: async () => 'system',
    chooseLanguage: async () => {},
    chooseTheme: async () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    install: async () => ({ outcome: 'started' }),
    onAutomaticSync: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    onSession: () => () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    filterLibrary: async (filter) => {
      asked.push(filter);
      return library.filter(
        (game) =>
          (filter.source === null || game.source === filter.source) &&
          (!filter.installedOnly || game.installed) &&
          game.title.toLowerCase().includes(filter.search.toLowerCase()),
      );
    },
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    game: async (gameId) => library.find((game) => game.id === gameId) ?? null,
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    coverHue: async () => null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
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
  };
  return { operations, asked };
}

/** The titles the Library shows, once it shows `count` Games. */
async function shownTitles(count: number) {
  const list = await screen.findByRole('list', { name: 'Games' });
  await within(list).findAllByRole('listitem');
  await expect.poll(() => within(list).getAllByRole('listitem').length).toBe(count);
  return within(list)
    .getAllByRole('heading')
    .map((heading) => heading.textContent);
}

describe('Filtering the Library', () => {
  it('shows the whole Library while no filter is set, without asking the core to filter', async () => {
    const { operations, asked } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    expect(await shownTitles(3)).toEqual(['Hades', 'Half-Life 2', 'Portal 2']);
    expect(screen.getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true');
    expect(asked).toEqual([]);
  });

  it('asks the core for the Games of the chosen Source', async () => {
    const user = userEvent.setup();
    const { operations, asked } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    const source = screen.getByRole('group', { name: 'Source' });
    await user.click(within(source).getByRole('button', { name: 'Steam' }));

    expect(asked.at(-1)).toEqual({ search: '', source: 'steam', installedOnly: false, status: null, sort: 'name', favoritesOnly: false });
    expect(within(source).getByRole('button', { name: 'Steam' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(source).getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('combines the filters with each other and with the search', async () => {
    const user = userEvent.setup();
    const { operations, asked } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.click(screen.getByRole('checkbox', { name: 'Installed only' }));
    expect(await shownTitles(2)).toEqual(['Hades', 'Portal 2']);

    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'ha');
    await user.click(screen.getByRole('button', { name: 'Steam' }));

    expect(asked.at(-1)).toEqual({ search: 'ha', source: 'steam', installedOnly: true, status: null, sort: 'name', favoritesOnly: false });
    expect(await shownTitles(1)).toEqual(['Hades']);
  });

  it('keeps the search and the filters when coming back from a detail page', async () => {
    const user = userEvent.setup();
    const { operations, asked } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);
    await user.click(screen.getByRole('checkbox', { name: 'Installed only' }));
    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'ha');
    expect(await shownTitles(1)).toEqual(['Hades']);
    const filtered = asked.length;

    await user.click(screen.getByRole('button', { name: 'Details for Hades' }));
    await screen.findByRole('region', { name: 'Hades' });
    await user.click(screen.getByRole('button', { name: 'Back to the library' }));

    expect(await shownTitles(1)).toEqual(['Hades']);
    expect(screen.getByRole('searchbox', { name: 'Search the library' })).toHaveValue('ha');
    expect(screen.getByRole('checkbox', { name: 'Installed only' })).toBeChecked();
    expect(asked.length).toBe(filtered);
  });

  it('shows an empty state when nothing matches, whose action clears every filter and the search', async () => {
    const user = userEvent.setup();
    const { operations, asked } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'half');
    await user.click(screen.getByRole('checkbox', { name: 'Installed only' }));

    expect(await screen.findByText('No games match these filters.')).toBeInTheDocument();
    expect(screen.queryByRole('list', { name: 'Games' })).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Reset filters' }));

    expect(await shownTitles(3)).toEqual(['Hades', 'Half-Life 2', 'Portal 2']);
    expect(screen.getByRole('searchbox', { name: 'Search the library' })).toHaveValue('');
    expect(screen.getByRole('checkbox', { name: 'Installed only' })).not.toBeChecked();
    expect(screen.getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByText('No games match these filters.')).not.toBeInTheDocument();
    expect(asked.at(-1)).toEqual({ search: 'half', source: null, installedOnly: true, status: null, sort: 'name', favoritesOnly: false });
  });

  it('shows the empty state for a Source without Games', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.click(screen.getByRole('button', { name: 'Local' }));

    expect(await screen.findByText('No games match these filters.')).toBeInTheDocument();
  });

  it('says so when the core could not filter', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    operations.filterLibrary = () => Promise.reject(new Error('database: disk I/O error'));
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.click(screen.getByRole('checkbox', { name: 'Installed only' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The library could not be filtered.');
  });

  it('speaks German', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);
    await screen.findByRole('list', { name: 'Spiele' });

    const source = screen.getByRole('group', { name: 'Quelle' });
    expect(within(source).getByRole('button', { name: 'Alle' })).toBeInTheDocument();
    expect(within(source).getByRole('button', { name: 'Steam' })).toBeInTheDocument();
    expect(screen.getByRole('searchbox', { name: 'Bibliothek durchsuchen' })).toBeInTheDocument();
    await user.click(within(source).getByRole('button', { name: 'Lokal' }));
    await user.click(screen.getByRole('checkbox', { name: 'Nur installierte' }));

    expect(await screen.findByText('Keine Spiele passen zu diesen Filtern.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Filter zurücksetzen' })).toBeInTheDocument();
  });
});

describe('Searching the Library by title', () => {
  const hollowKnight: Game = { ...steamGame(0, 'Hollow Knight', true), id: 'local:1', source: 'local' };
  const BOTH_SOURCES = [...LIBRARY, hollowKnight];

  it('asks the core with each typed letter and shows the Games of both Sources it answers', async () => {
    const user = userEvent.setup();
    const { operations, asked } = fakeOperations(BOTH_SOURCES);
    render(<App operations={operations} lang="en" />);
    await shownTitles(4);

    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'O');

    expect(asked.at(-1)).toEqual({ search: 'O', source: null, installedOnly: false, status: null, sort: 'name', favoritesOnly: false });
    expect(await shownTitles(2)).toEqual(['Portal 2', 'Hollow Knight']);

    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'L');

    expect(asked.map((filter) => filter.search)).toEqual(['O', 'OL']);
    expect(await shownTitles(1)).toEqual(['Hollow Knight']);
  });

  it('shows what the core answers instead of filtering in the interface', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    // A core that answers Portal 2 to any search, matching or not.
    operations.filterLibrary = async () => [LIBRARY[2]];
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'hades');

    expect(await shownTitles(1)).toEqual(['Portal 2']);
  });

  it('shows the full Library again once the search is cleared', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations(BOTH_SOURCES);
    render(<App operations={operations} lang="en" />);
    await shownTitles(4);
    const searchbox = screen.getByRole('searchbox', { name: 'Search the library' });

    await user.type(searchbox, 'hades');
    expect(await shownTitles(1)).toEqual(['Hades']);
    await user.clear(searchbox);

    expect(await shownTitles(4)).toEqual(['Hades', 'Half-Life 2', 'Portal 2', 'Hollow Knight']);
    expect(searchbox).toHaveValue('');
  });
});
