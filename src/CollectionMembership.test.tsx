import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Collection, Game, LibraryFilter, Operations } from './operations';

const PORTAL_2: Game = { id: 'steam:620', title: 'Portal 2', source: 'steam', installed: true, playtimeMinutes: 1260, lastPlayed: null, status: 'unplayed', favorite: false };
const HADES: Game = { id: 'local:1', title: 'Hades', source: 'local', installed: true, playtimeMinutes: null, lastPlayed: null, status: 'unplayed', favorite: false };
const PORTAL: Game = { id: 'steam:400', title: 'Portal', source: 'steam', installed: false, playtimeMinutes: 0, lastPlayed: null, status: 'unplayed', favorite: false };

/**
 * Fake operations whose Collections behave like the core's: a Game can be
 * in several Collections, the counts are the Games of the Library in each,
 * and the filter narrows to the chosen Collection; a Hidden Game is not in
 * the Library, so neither shown nor counted. `calls` records what
 * reached the core; `sync` takes the Games in `afterSync` as the Library.
 */
function fakeOperations({
  library: initial = [HADES, PORTAL, PORTAL_2],
  collections: named = [
    { id: 1, name: 'Backlog' },
    { id: 2, name: 'Co-op' },
  ],
  members: given = [],
  afterSync,
}: {
  library?: Game[];
  collections?: { id: number; name: string }[];
  members?: [number, string][];
  afterSync?: Game[];
} = {}) {
  let library = [...initial];
  let collections = [...named];
  let members = [...given];
  let hidden: Game[] = [];
  const calls = { added: [] as [number, string][], removed: [] as [number, string][], filters: [] as LibraryFilter[] };
  const inLibrary = (gameId: string) => library.some((game) => game.id === gameId);
  const listed = (): Collection[] =>
    collections.map((c) => ({
      ...c,
      gameCount: members.filter(([of, game]) => of === c.id && inLibrary(game)).length,
    }));
  const operations: Operations = {
    library: async () => library,
    readInstalledGames: async () => library,
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => true,
    storeSteamWebApiKey: async () => ({ outcome: 'invalid' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => {
      if (afterSync) library = [...afterSync];
      return { outcome: 'synced', at: 1_790_000_000 };
    },
    lastSync: async () => null,
    onAutomaticSync: () => () => {},
    filterLibrary: async (filter) => {
      calls.filters.push(filter);
      return library.filter(
        (game) =>
          game.title.toLowerCase().includes(filter.search.toLowerCase()) &&
          (!filter.installedOnly || game.installed) &&
          (filter.collection === undefined || members.some(([of, id]) => of === filter.collection && id === game.id)),
      );
    },
    game: async (gameId) => library.find((game) => game.id === gameId) ?? null,
    collections: async () => listed(),
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async (id, name) => {
      collections = collections.map((c) => (c.id === id ? { ...c, name } : c));
      return { outcome: 'named', id };
    },
    deleteCollection: async (id) => {
      collections = collections.filter((c) => c.id !== id);
      members = members.filter(([of]) => of !== id);
    },
    theme: async () => 'system',
    chooseTheme: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    gameCollections: async (gameId) =>
      collections.filter((c) => members.some(([of, game]) => of === c.id && game === gameId)).map((c) => c.id),
    addGameToCollection: async (collection, gameId) => {
      calls.added.push([collection, gameId]);
      if (!members.some(([of, game]) => of === collection && game === gameId)) members.push([collection, gameId]);
    },
    removeGameFromCollection: async (collection, gameId) => {
      calls.removed.push([collection, gameId]);
      members = members.filter(([of, game]) => !(of === collection && game === gameId));
    },
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    openLogFolder: async () => {},
    hiddenGames: async () => hidden,
    hideGame: async (gameId) => {
      hidden = [...hidden, ...library.filter((game) => game.id === gameId)];
      library = library.filter((game) => game.id !== gameId);
    },
    unhideGame: async (gameId) => {
      library = [...library, ...hidden.filter((game) => game.id === gameId)];
      hidden = hidden.filter((game) => game.id !== gameId);
    },
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    onSession: () => () => {},
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
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
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    coverHue: async () => null,
    language: async () => 'system',
    chooseLanguage: async () => {},
  };
  return { operations, calls };
}

/** The number of Games the sidebar shows for the Collection `name`. */
function sidebarCount(name: string) {
  const list = screen.getByRole('list', { name: /Collections|Sammlungen/ });
  const item = within(list).getByRole('button', { name }).closest('li')!;
  return item.querySelector('.collection-count [aria-hidden="true"]')?.textContent;
}

async function openDetails(title: string) {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: new RegExp(`^(Details for|Details zu) ${title}$`) }));
  return user;
}

/** The titles the Library grid shows. */
async function shown() {
  const grid = await screen.findByRole('list', { name: 'Games' });
  return within(grid)
    .getAllByRole('heading', { level: 2 })
    .map((heading) => heading.textContent);
}

describe('Collections on the detail page of a Game', () => {
  it('shows every Collection, checked for those the Game is in', async () => {
    const { operations } = fakeOperations({ members: [[2, PORTAL_2.id]] });
    render(<App operations={operations} lang="en" />);

    await openDetails('Portal 2');

    const group = await screen.findByRole('group', { name: 'Collections' });
    await waitFor(() => expect(within(group).getByRole('checkbox', { name: 'Co-op' })).toBeChecked());
    expect(within(group).getByRole('checkbox', { name: 'Backlog' })).not.toBeChecked();
  });

  it('puts the Game into several Collections at once and the sidebar counts follow at once', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const user = await openDetails('Portal 2');
    const group = await screen.findByRole('group', { name: 'Collections' });

    await user.click(within(group).getByRole('checkbox', { name: 'Backlog' }));
    await user.click(within(group).getByRole('checkbox', { name: 'Co-op' }));

    expect(calls.added).toEqual([
      [1, PORTAL_2.id],
      [2, PORTAL_2.id],
    ]);
    await waitFor(() => expect(within(group).getByRole('checkbox', { name: 'Co-op' })).toBeChecked());
    expect(within(group).getByRole('checkbox', { name: 'Backlog' })).toBeChecked();
    await waitFor(() => expect(sidebarCount('Co-op')).toBe('1'));
    expect(sidebarCount('Backlog')).toBe('1');
  });

  it('takes the Game out of a Collection and keeps it in the others', async () => {
    const { operations, calls } = fakeOperations({
      members: [
        [1, PORTAL_2.id],
        [2, PORTAL_2.id],
      ],
    });
    render(<App operations={operations} lang="en" />);
    const user = await openDetails('Portal 2');
    const group = await screen.findByRole('group', { name: 'Collections' });
    await waitFor(() => expect(within(group).getByRole('checkbox', { name: 'Backlog' })).toBeChecked());

    await user.click(within(group).getByRole('checkbox', { name: 'Backlog' }));

    expect(calls.removed).toEqual([[1, PORTAL_2.id]]);
    await waitFor(() => expect(sidebarCount('Backlog')).toBe('0'));
    expect(sidebarCount('Co-op')).toBe('1');
    expect(within(group).getByRole('checkbox', { name: 'Backlog' })).not.toBeChecked();
    expect(within(group).getByRole('checkbox', { name: 'Co-op' })).toBeChecked();
  });

  it('says so, in German too, when there is no Collection yet', async () => {
    const { operations } = fakeOperations({ collections: [] });
    render(<App operations={operations} lang="de" />);

    await openDetails('Portal 2');

    const group = await screen.findByRole('group', { name: 'Sammlungen' });
    expect(await within(group).findByText('Noch keine Sammlungen. Leg in der Seitenleiste eine an.')).toBeInTheDocument();
  });

  it('says so when the Game cannot be put into a Collection, and leaves it unchecked', async () => {
    const { operations } = fakeOperations();
    operations.addGameToCollection = () => Promise.reject(new Error('database: disk full'));
    render(<App operations={operations} lang="en" />);
    const user = await openDetails('Portal 2');
    const group = await screen.findByRole('group', { name: 'Collections' });

    await user.click(within(group).getByRole('checkbox', { name: 'Backlog' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The collections of Portal 2 could not be changed.');
    expect(within(group).getByRole('checkbox', { name: 'Backlog' })).not.toBeChecked();
  });

  it('shows a Collection created in the sidebar while the page is open', async () => {
    const { operations } = fakeOperations({ collections: [{ id: 1, name: 'Backlog' }] });
    let created = false;
    const list = operations.collections;
    operations.collections = async () =>
      created ? [...(await list()), { id: 3, name: 'Roguelikes', gameCount: 0 }] : list();
    operations.createCollection = async () => {
      created = true;
      return { outcome: 'named', id: 3 };
    };
    render(<App operations={operations} lang="en" />);
    const user = await openDetails('Portal 2');

    await user.click(screen.getByRole('button', { name: 'New collection' }));
    await user.type(screen.getByRole('textbox', { name: 'Name of the new collection' }), 'Roguelikes{Enter}');

    const group = screen.getByRole('group', { name: 'Collections' });
    expect(await within(group).findByRole('checkbox', { name: 'Roguelikes' })).not.toBeChecked();
  });
});

describe('Choosing a Collection in the sidebar', () => {
  it('narrows the Library to its Games under its name', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations({
      members: [
        [1, PORTAL_2.id],
        [1, HADES.id],
        [2, PORTAL.id],
      ],
    });
    render(<App operations={operations} lang="en" />);
    expect(await shown()).toEqual(['Hades', 'Portal', 'Portal 2']);

    await user.click(await screen.findByRole('button', { name: 'Backlog' }));

    expect(await screen.findByRole('heading', { level: 1, name: 'Backlog' })).toBeInTheDocument();
    await waitFor(async () => expect(await shown()).toEqual(['Hades', 'Portal 2']));
    expect(calls.filters.at(-1)).toEqual({ search: '', source: null, installedOnly: false, status: null, sort: 'name', favoritesOnly: false, collection: 1 });
    expect(screen.getByRole('button', { name: 'Backlog' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('button', { name: 'Library' })).not.toHaveAttribute('aria-current');
  });

  it('keeps the search and the filters within the Collection', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations({
      members: [
        [1, PORTAL_2.id],
        [1, HADES.id],
      ],
    });
    render(<App operations={operations} lang="en" />);
    await user.click(await screen.findByRole('button', { name: 'Backlog' }));

    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'port');

    await waitFor(async () => expect(await shown()).toEqual(['Portal 2']));
    expect(calls.filters.at(-1)).toEqual({ search: 'port', source: null, installedOnly: false, status: null, sort: 'name', favoritesOnly: false, collection: 1 });
  });

  it('shows the whole Library again from the navigation', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ members: [[1, PORTAL_2.id]] });
    render(<App operations={operations} lang="en" />);
    await user.click(await screen.findByRole('button', { name: 'Backlog' }));
    await waitFor(async () => expect(await shown()).toEqual(['Portal 2']));

    await user.click(screen.getByRole('button', { name: 'Library' }));

    expect(await screen.findByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    await waitFor(async () => expect(await shown()).toEqual(['Hades', 'Portal', 'Portal 2']));
    expect(screen.getByRole('button', { name: 'Library' })).toHaveAttribute('aria-current', 'page');
  });

  it('says so, in German too, when the Collection has no Game yet', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);

    await user.click(await screen.findByRole('button', { name: 'Co-op' }));

    expect(
      await screen.findByText('In dieser Sammlung ist noch kein Spiel. Füg Spiele auf ihrer Detailseite hinzu.'),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Keine Spiele passen/)).not.toBeInTheDocument();
  });

  it('drops a Game taken out of the Collection on its detail page', async () => {
    const { operations } = fakeOperations({
      members: [
        [1, PORTAL_2.id],
        [1, HADES.id],
      ],
    });
    render(<App operations={operations} lang="en" />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Backlog' }));
    await waitFor(async () => expect(await shown()).toEqual(['Hades', 'Portal 2']));
    await openDetails('Portal 2');
    const group = await screen.findByRole('group', { name: 'Collections' });
    await waitFor(() => expect(within(group).getByRole('checkbox', { name: 'Backlog' })).toBeChecked());

    await user.click(within(group).getByRole('checkbox', { name: 'Backlog' }));
    await waitFor(() => expect(sidebarCount('Backlog')).toBe('1'));
    await user.click(screen.getByRole('button', { name: 'Back to the library' }));

    await waitFor(async () => expect(await shown()).toEqual(['Hades']));
  });

  it('shows the new name of the chosen Collection once it is renamed', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await user.click(await screen.findByRole('button', { name: 'Backlog' }));

    await user.click(screen.getByRole('button', { name: 'Rename Backlog' }));
    const field = screen.getByRole('textbox', { name: 'New name for Backlog' });
    await user.clear(field);
    await user.type(field, 'Pile of shame{Enter}');

    expect(await screen.findByRole('heading', { level: 1, name: 'Pile of shame' })).toBeInTheDocument();
  });

  it('shows the whole Library again once the chosen Collection is deleted', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ members: [[1, PORTAL_2.id]] });
    render(<App operations={operations} lang="en" />);
    await user.click(await screen.findByRole('button', { name: 'Backlog' }));
    await waitFor(async () => expect(await shown()).toEqual(['Portal 2']));

    await user.click(screen.getByRole('button', { name: 'Delete Backlog' }));
    await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Delete' }));

    expect(await screen.findByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    await waitFor(async () => expect(await shown()).toEqual(['Hades', 'Portal', 'Portal 2']));
  });
});

describe('Hiding a Game', () => {
  it('takes it out of the chosen Collection and its sidebar count until it is unhidden', async () => {
    const { operations } = fakeOperations({
      members: [
        [1, PORTAL_2.id],
        [1, HADES.id],
      ],
    });
    render(<App operations={operations} lang="en" />);
    await waitFor(() => expect(sidebarCount('Backlog')).toBe('2'));
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Backlog' }));
    await waitFor(async () => expect(await shown()).toEqual(['Hades', 'Portal 2']));

    await openDetails('Portal 2');
    await user.click(await screen.findByRole('button', { name: 'Hide Portal 2' }));

    await waitFor(() => expect(sidebarCount('Backlog')).toBe('1'));
    await waitFor(async () => expect(await shown()).toEqual(['Hades']));

    await user.click(screen.getByRole('button', { name: 'Hidden games' }));
    await user.click(await screen.findByRole('button', { name: 'Unhide Portal 2' }));
    await waitFor(() => expect(sidebarCount('Backlog')).toBe('2'));
  });
});

describe('After a Sync', () => {
  it('the sidebar counts are read again', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({
      members: [
        [1, PORTAL_2.id],
        [1, PORTAL.id],
      ],
      // The Sync takes Portal out of the Library.
      afterSync: [HADES, PORTAL_2],
    });
    render(<App operations={operations} lang="en" />);
    await waitFor(() => expect(sidebarCount('Backlog')).toBe('2'));

    await user.click(screen.getByRole('button', { name: 'Sync now' }));

    await waitFor(() => expect(sidebarCount('Backlog')).toBe('1'));
  });
});
