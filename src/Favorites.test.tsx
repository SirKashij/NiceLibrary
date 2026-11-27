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
 * Fake operations that keep Favorites like the core: the Library answers
 * which Games are marked, filters to them and counts them; a Hidden Game is
 * neither in the Library nor counted. `marking` makes `markFavorite` fail
 * when set to `'fails'`.
 */
function fakeOperations({ favorites = [], marking = 'works' }: { favorites?: string[]; marking?: 'works' | 'fails' } = {}) {
  const marked = new Set(favorites);
  const hidden = new Set<string>();
  const asked: LibraryFilter[] = [];
  const marks: [string, boolean][] = [];
  const library = () =>
    LIBRARY.filter((game) => !hidden.has(game.id)).map((game) => ({ ...game, favorite: marked.has(game.id) }));
  const operations: Operations = {
    library: async () => library(),
    readInstalledGames: async () => library(),
    steamInstallation: async () => ({ state: 'notFound' }),
    exportData: async () => ({ outcome: 'cancelled' }),
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
    onAutomaticSync: () => () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    filterLibrary: async (filter) => {
      asked.push(filter);
      return library().filter(
        (game) =>
          (filter.source === null || game.source === filter.source) &&
          (!filter.installedOnly || game.installed) &&
          (!filter.favoritesOnly || game.favorite) &&
          game.title.toLowerCase().includes(filter.search.toLowerCase()),
      );
    },
    game: async (gameId) => library().find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    markFavorite: async (gameId, favorite) => {
      marks.push([gameId, favorite]);
      if (marking === 'fails') throw new Error('database: disk I/O error');
      if (favorite) marked.add(gameId);
      else marked.delete(gameId);
    },
    favoriteCount: async () => library().filter((game) => game.favorite).length,
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    openLogFolder: async () => {},
    hiddenGames: async () => LIBRARY.filter((game) => hidden.has(game.id)).map((game) => ({ ...game, favorite: marked.has(game.id) })),
    hideGame: async (gameId) => {
      hidden.add(gameId);
    },
    unhideGame: async (gameId) => {
      hidden.delete(gameId);
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
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    install: async () => ({ outcome: 'started' }),
    coverHue: async () => null,
    language: async () => 'system',
    chooseLanguage: async () => {},
  };
  return { operations, asked, marks };
}

/** The card of the Game `title` in the Library. */
async function card(title: string) {
  const list = await screen.findByRole('list', { name: 'Games' });
  const heading = await within(list).findByRole('heading', { name: title });
  return heading.closest('li') as HTMLElement;
}

/** The titles the Library shows, once it shows `count` Games. */
async function shownTitles(count: number) {
  const list = await screen.findByRole('list', { name: 'Games' });
  await expect.poll(() => within(list).queryAllByRole('listitem').length).toBe(count);
  return within(list)
    .getAllByRole('heading')
    .map((heading) => heading.textContent);
}

/** The Favorites entry in the sidebar. */
function favoritesEntry() {
  return screen.getByRole('button', { name: /^Favorites/ });
}

describe('Favorites', () => {
  it('marks a Game as a Favorite from its card and shows the marker on its Cover', async () => {
    const user = userEvent.setup();
    const { operations, marks } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    const portal = await card('Portal 2');
    const toggle = within(portal).getByRole('button', { name: 'Favorite Portal 2' });
    expect(toggle).toHaveAttribute('aria-pressed', 'false');

    await user.click(toggle);

    expect(marks).toEqual([['steam:620', true]]);
    await expect.poll(() => within(portal).getByRole('button', { name: 'Favorite Portal 2' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(await card('Hades')).getByRole('button', { name: 'Favorite Hades' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('shows the Favorites marked before, and unmarks one from its card', async () => {
    const user = userEvent.setup();
    const { operations, marks } = fakeOperations({ favorites: ['steam:220'] });
    render(<App operations={operations} lang="en" />);

    const halfLife = await card('Half-Life 2');
    const toggle = await within(halfLife).findByRole('button', { name: 'Favorite Half-Life 2', pressed: true });

    await user.click(toggle);

    expect(marks).toEqual([['steam:220', false]]);
    await within(halfLife).findByRole('button', { name: 'Favorite Half-Life 2', pressed: false });
  });

  it('counts the Favorites in the sidebar and keeps the count up to date', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ favorites: ['steam:220'] });
    render(<App operations={operations} lang="en" />);

    await expect.poll(() => favoritesEntry()).toHaveAccessibleName('Favorites, 1 game');

    await user.click(within(await card('Hades')).getByRole('button', { name: 'Favorite Hades' }));

    await expect.poll(() => favoritesEntry()).toHaveAccessibleName('Favorites, 2 games');
  });

  it('counts a hidden Favorite again only once it is unhidden', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ favorites: ['steam:220', 'steam:620'] });
    render(<App operations={operations} lang="en" />);
    await expect.poll(() => favoritesEntry()).toHaveAccessibleName('Favorites, 2 games');

    await user.click(within(await card('Portal 2')).getByRole('button', { name: 'Details for Portal 2' }));
    await user.click(await screen.findByRole('button', { name: 'Hide Portal 2' }));
    await expect.poll(() => favoritesEntry()).toHaveAccessibleName('Favorites, 1 game');

    await user.click(screen.getByRole('button', { name: 'Hidden games' }));
    await user.click(await screen.findByRole('button', { name: 'Unhide Portal 2' }));
    await expect.poll(() => favoritesEntry()).toHaveAccessibleName('Favorites, 2 games');
  });

  it('narrows the Library to the Favorites from the sidebar, and back to the whole Library', async () => {
    const user = userEvent.setup();
    const { operations, asked } = fakeOperations({ favorites: ['steam:220', 'steam:620'] });
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.click(favoritesEntry());

    expect(asked.at(-1)).toEqual({ search: '', source: null, installedOnly: false, status: null, sort: 'name', favoritesOnly: true });
    expect(await shownTitles(2)).toEqual(['Half-Life 2', 'Portal 2']);
    expect(screen.getByRole('heading', { level: 1, name: 'Favorites' })).toBeInTheDocument();
    expect(favoritesEntry()).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('button', { name: 'Library' })).not.toHaveAttribute('aria-current');

    await user.click(screen.getByRole('checkbox', { name: 'Installed only' }));
    expect(await shownTitles(1)).toEqual(['Portal 2']);

    await user.click(screen.getByRole('button', { name: 'Library' }));

    expect(screen.getByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    expect(asked.at(-1)).toEqual({ search: '', source: null, installedOnly: true, status: null, sort: 'name', favoritesOnly: false });
    expect(await shownTitles(2)).toEqual(['Hades', 'Portal 2']);
  });

  it('takes a Game out of the Favorites view once it is unmarked there', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ favorites: ['steam:220', 'steam:620'] });
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);
    await user.click(favoritesEntry());
    await shownTitles(2);

    await user.click(within(await card('Portal 2')).getByRole('button', { name: 'Favorite Portal 2' }));

    expect(await shownTitles(1)).toEqual(['Half-Life 2']);
  });

  it('says how to mark a Favorite when there is none yet', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.click(favoritesEntry());

    expect(await screen.findByText('No favorites yet. Mark a game with its star to find it here.')).toBeInTheDocument();
    expect(screen.queryByRole('list', { name: 'Games' })).not.toBeInTheDocument();
  });

  it('marks and unmarks a Game on its detail page', async () => {
    const user = userEvent.setup();
    const { operations, marks } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(within(await card('Portal 2')).getByRole('button', { name: 'Details for Portal 2' }));
    const detail = await screen.findByRole('region', { name: 'Portal 2' });
    await user.click(within(detail).getByRole('button', { name: 'Favorite Portal 2', pressed: false }));

    await within(detail).findByRole('button', { name: 'Favorite Portal 2', pressed: true });
    await expect.poll(() => favoritesEntry()).toHaveAccessibleName('Favorites, 1 game');

    await user.click(within(detail).getByRole('button', { name: 'Favorite Portal 2' }));

    await within(detail).findByRole('button', { name: 'Favorite Portal 2', pressed: false });
    expect(marks).toEqual([
      ['steam:620', true],
      ['steam:620', false],
    ]);

    await user.click(screen.getByRole('button', { name: 'Back to the library' }));
    expect(within(await card('Portal 2')).getByRole('button', { name: 'Favorite Portal 2' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('says so when a Game could not be marked, and keeps it as it was', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ marking: 'fails' });
    render(<App operations={operations} lang="en" />);

    await user.click(within(await card('Portal 2')).getByRole('button', { name: 'Favorite Portal 2' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Portal 2 could not be marked or unmarked as a favorite.');
    expect(within(await card('Portal 2')).getByRole('button', { name: 'Favorite Portal 2' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('speaks German', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ favorites: ['steam:620'] });
    render(<App operations={operations} lang="de" />);

    const list = await screen.findByRole('list', { name: 'Spiele' });
    const portal = (await within(list).findByRole('heading', { name: 'Portal 2' })).closest('li') as HTMLElement;
    await within(portal).findByRole('button', { name: 'Portal 2 als Favorit', pressed: true });
    const entry = screen.getByRole('button', { name: /^Favoriten/ });
    await expect.poll(() => entry).toHaveAccessibleName('Favoriten, 1 Spiel');

    await user.click(entry);

    expect(screen.getByRole('heading', { level: 1, name: 'Favoriten' })).toBeInTheDocument();
  });
});
