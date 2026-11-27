import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, Operations, SteamTitle } from './operations';

const EXECUTABLE = 'C:\\Games\\Hollow Knight\\hollow_knight.exe';

/** The Steam App List as the fake core stores it. */
const STEAM_APP_LIST: SteamTitle[] = [
  { appId: 367520, name: 'Hollow Knight' },
  { appId: 1145360, name: 'Hades' },
  { appId: 1145350, name: 'Hades II' },
  { appId: 504230, name: 'Celeste' },
];

function localGame(id: number, title: string): Game {
  return { id: `local:${id}`, title, source: 'local', installed: true, playtimeMinutes: 0, lastPlayed: null, status: 'unplayed', favorite: false, progress: null };
}

const PORTAL_2: Game = { ...localGame(0, 'Portal 2'), id: 'steam:620', source: 'steam', playtimeMinutes: null };

/**
 * Fake operations whose core keeps Cover Matches as the real one does.
 * `library` is the Library at the start; the user chooses the Hollow Knight
 * executable when adding. `appList` is the stored Steam App List, empty as
 * without a key. Each `setCoverMatch` is recorded.
 */
function fakeOperations({
  library = [] as Game[],
  appList = STEAM_APP_LIST,
  matches = {} as Record<string, SteamTitle>,
} = {}) {
  const games = [...library];
  const matched = new Map(Object.entries(matches));
  const calls = { setCoverMatch: [] as [string, number | null][] };
  const searchable = (text: string) => text.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, ' ').trim();
  const operations: Operations = {
    library: async () => [...games],
    readInstalledGames: async () => [...games],
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game, revision) => `cover://localhost/${game.id}${revision ? `?revision=${revision}` : ''}`,
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => appList.length > 0,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    openLogFolder: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    onAutomaticSync: () => () => {},
    steamAppList: async () => ({ titles: appList.length, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    theme: async () => 'system',
    chooseTheme: async () => {},
    language: async () => 'system',
    chooseLanguage: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    onSession: () => () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    filterLibrary: async () => [...games],
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    achievements: async () => ({ state: 'localGame' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    chooseLocalGameExecutable: async () => ({ outcome: 'chosen', executable: EXECUTABLE, title: 'Hollow Knight' }),
    addLocalGame: async (_executable, title) => {
      const game = localGame(games.length + 1, title.trim());
      games.push(game);
      return { outcome: 'added', game };
    },
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    findSteamTitles: async (typed) =>
      searchable(typed) === '' ? [] : appList.filter((title) => searchable(title.name).includes(searchable(typed))),
    proposeCoverMatch: async (title) => {
      const search = searchable(title);
      if (search === '') return null;
      return appList.find((steam) => searchable(steam.name) === search) ?? null;
    },
    coverMatch: async (gameId) => matched.get(gameId) ?? null,
    importData: async () => ({ outcome: 'cancelled' }),
    setCoverMatch: async (gameId, appId) => {
      calls.setCoverMatch.push([gameId, appId]);
      const title = appList.find((steam) => steam.appId === appId);
      if (title) matched.set(gameId, title);
      else matched.delete(gameId);
    },
    coverHue: async () => null,
    install: async () => ({ outcome: 'started' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    exportData: async () => ({ outcome: 'cancelled' }),
  };
  return { operations, calls };
}

async function startAdding() {
  const user = userEvent.setup();
  await user.click(await screen.findByRole('button', { name: /Add a game|Spiel hinzufügen/ }));
  const form = await screen.findByRole('form', { name: /Add a game|Spiel hinzufügen/ });
  return { user, form };
}

async function openDetail(title: string) {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: 'Games' });
  await user.click(within(grid).getByRole('button', { name: `Details for ${title}` }));
  const page = await screen.findByRole('region', { name: title });
  return { user, page };
}

describe('The Cover Match proposed when adding a Local Game', () => {
  it('shows the best matching Steam title with its Cover before the Game is saved', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    const { form } = await startAdding();

    expect(await within(form).findByText('Cover from Steam: Hollow Knight')).toBeInTheDocument();
    expect(within(form).getByRole('img', { name: 'Cover of Hollow Knight' })).toHaveAttribute(
      'src',
      'cover://localhost/steam:367520',
    );
    expect(calls.setCoverMatch).toEqual([]);
  });

  it('is confirmed by adding the Game', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, form } = await startAdding();
    await within(form).findByText('Cover from Steam: Hollow Knight');

    await user.click(within(form).getByRole('button', { name: 'Add to library' }));

    expect(await screen.findByText('Hollow Knight was added to your library.')).toBeInTheDocument();
    expect(calls.setCoverMatch).toEqual([['local:1', 367520]]);
  });

  it('can be replaced by another title the user searches for in the Steam App List', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, form } = await startAdding();
    await user.click(await within(form).findByRole('button', { name: 'Choose another title…' }));

    await user.type(within(form).getByRole('searchbox', { name: 'Search Steam titles' }), 'hades');
    const results = await within(form).findByRole('list', { name: 'Steam titles' });
    expect(within(results).getAllByRole('button').map((button) => button.textContent)).toEqual(['Hades', 'Hades II']);
    await user.click(within(results).getByRole('button', { name: 'Hades II' }));

    expect(within(form).getByText('Cover from Steam: Hades II')).toBeInTheDocument();
    await user.click(within(form).getByRole('button', { name: 'Add to library' }));
    await screen.findByText('Hollow Knight was added to your library.');
    expect(calls.setCoverMatch).toEqual([['local:1', 1145350]]);
  });

  it('can be declined, and the Game keeps its icon Cover', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, form } = await startAdding();

    await user.click(await within(form).findByRole('button', { name: 'Use the icon instead' }));

    expect(within(form).getByText('The game shows its executable’s icon.')).toBeInTheDocument();
    expect(within(form).queryByRole('img')).not.toBeInTheDocument();
    await user.click(within(form).getByRole('button', { name: 'Add to library' }));
    await screen.findByText('Hollow Knight was added to your library.');
    expect(calls.setCoverMatch).toEqual([]);
  });

  it('follows the title the user corrects, until they choose a title themselves', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, form } = await startAdding();
    await within(form).findByText('Cover from Steam: Hollow Knight');

    const title = within(form).getByRole('textbox', { name: 'Title' });
    await user.clear(title);
    await user.type(title, 'Celeste');

    expect(await within(form).findByText('Cover from Steam: Celeste')).toBeInTheDocument();
  });

  it('is not there without a match, and the Game keeps its icon Cover', async () => {
    const { operations, calls } = fakeOperations({ appList: [] });
    render(<App operations={operations} lang="en" />);
    const { user, form } = await startAdding();

    expect(
      await within(form).findByText('No Steam title matches. The game shows its executable’s icon.'),
    ).toBeInTheDocument();
    await user.click(within(form).getByRole('button', { name: 'Choose a Steam title…' }));
    await user.type(within(form).getByRole('searchbox', { name: 'Search Steam titles' }), 'hollow');
    expect(
      await within(form).findByText(
        'No Steam title found. Without a Steam Web API key the app has no list of Steam titles.',
      ),
    ).toBeInTheDocument();
    await user.click(within(form).getByRole('button', { name: 'Add to library' }));
    await screen.findByText('Hollow Knight was added to your library.');
    expect(calls.setCoverMatch).toEqual([]);
  });

  it('speaks German in the German interface', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);
    const { form } = await startAdding();

    expect(await within(form).findByText('Cover von Steam: Hollow Knight')).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Anderen Titel wählen …' })).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Stattdessen das Symbol verwenden' })).toBeInTheDocument();
  });
});

describe('The Cover Match on the detail page of a Local Game', () => {
  it('shows the matched title and can be changed', async () => {
    const hollowKnight = localGame(1, 'Hollow Knight');
    const { operations, calls } = fakeOperations({
      library: [hollowKnight],
      matches: { 'local:1': STEAM_APP_LIST[0] },
    });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Hollow Knight');
    expect(await within(page).findByText('Cover from Steam: Hollow Knight')).toBeInTheDocument();

    await user.click(within(page).getByRole('button', { name: 'Choose another title…' }));
    await user.type(within(page).getByRole('searchbox', { name: 'Search Steam titles' }), 'hades');
    const results = await within(page).findByRole('list', { name: 'Steam titles' });
    await user.click(within(results).getByRole('button', { name: 'Hades' }));

    expect(calls.setCoverMatch).toEqual([['local:1', 1145360]]);
    expect(await within(page).findByText('Cover from Steam: Hades')).toBeInTheDocument();
    expect(within(page).getByRole('img', { name: 'Cover of Hollow Knight' })).toHaveAttribute(
      'src',
      'cover://localhost/local:1?revision=1',
    );
  });

  it('can be removed, and the Game shows its icon Cover again', async () => {
    const { operations, calls } = fakeOperations({
      library: [localGame(1, 'Hollow Knight')],
      matches: { 'local:1': STEAM_APP_LIST[0] },
    });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Hollow Knight');

    await user.click(await within(page).findByRole('button', { name: 'Remove the Steam cover' }));

    expect(calls.setCoverMatch).toEqual([['local:1', null]]);
    expect(await within(page).findByRole('button', { name: 'Choose a Steam title…' })).toBeInTheDocument();
    expect(within(page).getByRole('img', { name: 'Cover of Hollow Knight' })).toHaveAttribute(
      'src',
      'cover://localhost/local:1?revision=1',
    );
  });

  it('can be made later for a Game without one', async () => {
    const { operations, calls } = fakeOperations({ library: [localGame(1, 'Hollow Knight')] });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Hollow Knight');

    await user.click(await within(page).findByRole('button', { name: 'Choose a Steam title…' }));
    await user.type(within(page).getByRole('searchbox', { name: 'Search Steam titles' }), 'hollow');
    await user.click(
      within(await within(page).findByRole('list', { name: 'Steam titles' })).getByRole('button', {
        name: 'Hollow Knight',
      }),
    );

    expect(calls.setCoverMatch).toEqual([['local:1', 367520]]);
    expect(await within(page).findByText('Cover from Steam: Hollow Knight')).toBeInTheDocument();
  });

  it('is not offered for a Steam Game', async () => {
    const { operations } = fakeOperations({ library: [PORTAL_2] });
    render(<App operations={operations} lang="en" />);
    const { page } = await openDetail('Portal 2');

    expect(within(page).queryByRole('button', { name: 'Choose a Steam title…' })).not.toBeInTheDocument();
    expect(within(page).queryByText(/Cover from Steam/)).not.toBeInTheDocument();
  });
});
