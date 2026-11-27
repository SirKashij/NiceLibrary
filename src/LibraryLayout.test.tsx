import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LibraryLayout, Operations } from './operations';

const LIBRARY: Game[] = [
  { id: 'steam:1145360', title: 'Hades', source: 'steam', installed: true, playtimeMinutes: 600, lastPlayed: 1_700_000_000, status: 'playing', favorite: true, progress: null },
  { id: 'steam:220', title: 'Half-Life 2', source: 'steam', installed: false, playtimeMinutes: 0, lastPlayed: null, status: 'unplayed', favorite: false, progress: null },
  { id: 'local:1', title: 'Portal 2', source: 'local', installed: true, playtimeMinutes: null, lastPlayed: null, status: 'unplayed', favorite: false, progress: null },
];

/** A promise that never settles: the operation is still running. */
function pending<T>(): Promise<T> {
  return new Promise<T>(() => {});
}

/**
 * Fake operations over LIBRARY that start with `stored` as the stored Library
 * Layout. Records each Library Layout chosen, each Game launched and each
 * Favorite mark.
 */
function fakeOperations(stored: Promise<LibraryLayout> = Promise.resolve('coverGrid')) {
  const chosen: LibraryLayout[] = [];
  const launched: string[] = [];
  const marks: [string, boolean][] = [];
  let failChoosing = false;
  const operations: Operations = {
    library: async () => LIBRARY,
    readInstalledGames: async () => LIBRARY,
    steamInstallation: async () => ({ state: 'notFound' }),
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async (game) => {
      launched.push(game.id);
      return { outcome: 'launched' };
    },
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
    filterLibrary: async () => LIBRARY,
    game: async (gameId) => LIBRARY.find((game) => game.id === gameId) ?? null,
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
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    libraryLayout: () => stored,
    chooseLibraryLayout: async (layout) => {
      chosen.push(layout);
      if (failChoosing) throw new Error('the settings could not be written');
    },
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    openLogFolder: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    onSession: () => () => {},
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
    markFavorite: async (gameId, favorite) => {
      marks.push([gameId, favorite]);
    },
    favoriteCount: async () => 0,
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    language: async () => 'system',
    chooseLanguage: async () => {},
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
  };
  return { operations, chosen, launched, marks, failWhenChoosing: () => (failChoosing = true) };
}

/** The Library's Games as shown, once it shows all of LIBRARY. */
async function shownGames() {
  const list = await screen.findByRole('list', { name: 'Games' });
  await expect.poll(() => within(list).queryAllByRole('listitem').length).toBe(LIBRARY.length);
  return within(list).getAllByRole('listitem');
}

/** The titles of the shown Games, in the order shown. */
async function shownTitles() {
  return (await shownGames()).map((game) => within(game).getByRole('heading').textContent);
}

/** Whether the Library shows its Games as the list: each row names the Game's Source. */
async function showsTheList() {
  const [hades] = await shownGames();
  return within(hades).queryByText('Steam') !== null;
}

describe('Switching between the cover grid and the list', () => {
  it('shows the cover grid on first start', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    expect(await showsTheList()).toBe(false);
    const layout = screen.getByRole('group', { name: 'Layout' });
    expect(within(layout).getByRole('button', { name: 'Grid' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(layout).getByRole('button', { name: 'List' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('shows the list when the list was chosen before the restart', async () => {
    const { operations } = fakeOperations(Promise.resolve('list'));
    render(<App operations={operations} lang="en" />);

    expect(await showsTheList()).toBe(true);
    expect(screen.getByRole('button', { name: 'List' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('shows no Games until the stored Library Layout answers, so the other layout never flashes', async () => {
    const { operations } = fakeOperations(pending());
    render(<App operations={operations} lang="en" />);

    await screen.findByRole('group', { name: 'Layout' });
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(screen.queryByRole('list', { name: 'Games' })).not.toBeInTheDocument();
  });

  it('shows the cover grid when the stored Library Layout cannot be read', async () => {
    const { operations } = fakeOperations(Promise.reject(new Error('the settings could not be read')));
    render(<App operations={operations} lang="en" />);

    expect(await showsTheList()).toBe(false);
  });

  it('shows the same Games in the same order in both layouts and keeps each choice', async () => {
    const user = userEvent.setup();
    const { operations, chosen } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const inTheGrid = await shownTitles();

    await user.click(screen.getByRole('button', { name: 'List' }));
    expect(await showsTheList()).toBe(true);
    expect(await shownTitles()).toEqual(inTheGrid);
    expect(inTheGrid).toEqual(['Hades', 'Half-Life 2', 'Portal 2']);

    await user.click(screen.getByRole('button', { name: 'Grid' }));
    expect(await showsTheList()).toBe(false);
    expect(await shownTitles()).toEqual(inTheGrid);
    expect(chosen).toEqual(['list', 'coverGrid']);
  });

  it('goes back to the layout before and says so when the choice cannot be kept', async () => {
    const user = userEvent.setup();
    const { operations, failWhenChoosing } = fakeOperations();
    failWhenChoosing();
    render(<App operations={operations} lang="en" />);
    await shownGames();

    await user.click(screen.getByRole('button', { name: 'List' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The layout could not be saved.');
    expect(await showsTheList()).toBe(false);
    expect(screen.getByRole('button', { name: 'Grid' })).toHaveAttribute('aria-pressed', 'true');
  });
});

describe('The list', () => {
  it('shows one row per Game with its title, Source, Playtime, Last Played and main action', async () => {
    const { operations } = fakeOperations(Promise.resolve('list'));
    render(<App operations={operations} lang="en" />);
    const [hades, halfLife, portal] = await shownGames();

    expect(within(hades).getByRole('img', { name: 'Cover of Hades' })).toBeInTheDocument();
    expect(within(hades).getByText('Steam')).toBeInTheDocument();
    expect(within(hades).getByText('10 h')).toBeInTheDocument();
    expect(within(hades).getByText('Nov 14, 2023')).toBeInTheDocument();
    expect(within(hades).getByRole('button', { name: 'Play Hades' })).toBeInTheDocument();

    expect(within(halfLife).getByText('Never played')).toBeInTheDocument();
    expect(within(halfLife).getByText('Never')).toBeInTheDocument();
    expect(within(halfLife).getByText('Not installed')).toBeInTheDocument();
    expect(within(halfLife).queryByRole('button', { name: /^Play/ })).not.toBeInTheDocument();

    expect(within(portal).getByText('Local')).toBeInTheDocument();
    expect(within(portal).getAllByText('Not known yet')).toHaveLength(2);
  });

  it('shows the Status of each row and marks a Game as a Favorite with its star', async () => {
    const user = userEvent.setup();
    const { operations, marks } = fakeOperations(Promise.resolve('list'));
    render(<App operations={operations} lang="en" />);
    const [hades, halfLife] = await shownGames();

    expect(within(hades).getByText('Playing')).toBeInTheDocument();
    expect(within(halfLife).getByText('Unplayed')).toBeInTheDocument();
    expect(within(hades).getByRole('button', { name: 'Favorite Hades' })).toHaveAttribute('aria-pressed', 'true');
    const star = within(halfLife).getByRole('button', { name: 'Favorite Half-Life 2' });
    expect(star).toHaveAttribute('aria-pressed', 'false');

    await user.click(star);

    expect(marks).toEqual([['steam:220', true]]);
    // The star marks; it does not open the detail page.
    expect(screen.queryByRole('button', { name: 'Back to the library' })).not.toBeInTheDocument();
  });

  it('opens the detail page of a row, and Play launches the Game, by keyboard', async () => {
    const user = userEvent.setup();
    const { operations, launched } = fakeOperations(Promise.resolve('list'));
    render(<App operations={operations} lang="en" />);
    await shownGames();

    screen.getByRole('button', { name: 'Play Hades' }).focus();
    await user.keyboard('{Enter}');
    expect(launched).toEqual(['steam:1145360']);

    screen.getByRole('button', { name: 'Details for Portal 2' }).focus();
    await user.keyboard('{Enter}');
    expect(await screen.findByRole('heading', { level: 1, name: 'Portal 2' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Back to the library' }));
    expect(await showsTheList()).toBe(true);
    expect(screen.getByRole('button', { name: 'Details for Portal 2' })).toHaveFocus();
  });

  it('is reached row by row with Tab, after the layout switch', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations(Promise.resolve('list'));
    render(<App operations={operations} lang="en" />);
    await shownGames();

    screen.getByRole('button', { name: 'List' }).focus();
    const reached: (string | null)[] = [];
    for (let i = 0; i < 8; i++) {
      await user.tab();
      reached.push(document.activeElement?.getAttribute('aria-label') ?? null);
    }
    expect(reached).toEqual([
      'Details for Hades',
      'Favorite Hades',
      'Play Hades',
      'Details for Half-Life 2',
      'Favorite Half-Life 2',
      'Details for Portal 2',
      'Favorite Portal 2',
      'Play Portal 2',
    ]);
  });

  it('speaks German in the German interface', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);
    await screen.findByRole('list', { name: 'Spiele' });

    const layout = screen.getByRole('group', { name: 'Ansicht' });
    expect(within(layout).getByRole('button', { name: 'Raster' })).toHaveAttribute('aria-pressed', 'true');
    await user.click(within(layout).getByRole('button', { name: 'Liste' }));

    const [hades] = within(screen.getByRole('list', { name: 'Spiele' })).getAllByRole('listitem');
    expect(within(hades).getByText('10 Std.')).toBeInTheDocument();
    expect(within(hades).getByText('14.11.2023')).toBeInTheDocument();
    expect(within(hades).getByRole('button', { name: 'Hades spielen' })).toBeInTheDocument();
  });
});
