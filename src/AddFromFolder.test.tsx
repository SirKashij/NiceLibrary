import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { AddLocalGame, FolderSuggestions, Game, Operations } from './operations';

const FOLDER = 'C:\\Games';
const CELESTE = 'C:\\Games\\Celeste\\Celeste.exe';
const HADES = 'C:\\Games\\Hades\\x64\\Hades.exe';
const PORTAL = 'C:\\Games\\Portal 2\\portal2.exe';

function localGame(id: number, title: string): Game {
  return { id: `local:${id}`, title, source: 'local', installed: true, playtimeMinutes: 0, lastPlayed: null, status: 'unplayed', favorite: false };
}

/** The folder the user chooses: Celeste and Hades are new, Portal 2 is in the Library already. */
const GAMES_FOLDER: FolderSuggestions = {
  outcome: 'suggested',
  folder: FOLDER,
  executables: [
    { executable: CELESTE, title: 'Celeste', alreadyAdded: false },
    { executable: HADES, title: 'Hades', alreadyAdded: false },
    { executable: PORTAL, title: 'Portal 2', alreadyAdded: true },
  ],
};

/**
 * Fake operations over a Library that starts empty. `folder` is what the
 * user does in the folder dialog; `add` answers each Local Game to add, by
 * default adding it. `added` records what was asked to be added, `matched`
 * each Cover Match saved.
 */
function fakeOperations({
  folder = async () => GAMES_FOLDER,
  add,
}: {
  folder?: () => Promise<FolderSuggestions>;
  add?: (executable: string, title: string) => Promise<AddLocalGame>;
} = {}) {
  const games: Game[] = [];
  const added: { executable: string; title: string }[] = [];
  const matched: [string, number | null][] = [];
  const operations: Operations = {
    library: async () => [...games],
    readInstalledGames: async () => [...games],
    steamInstallation: async () => ({ state: 'notFound' }),
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
    onAutomaticSync: () => () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    filterLibrary: async () => [...games],
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    suggestExecutablesInFolder: folder,
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    addLocalGame: async (executable, title) => {
      added.push({ executable, title });
      if (add) return add(executable, title);
      const game = localGame(games.length + 1, title.trim());
      games.push(game);
      return { outcome: 'added', game };
    },
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    language: async () => 'system',
    chooseLanguage: async () => {},
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    onSession: () => () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    coverHue: async () => null,
    install: async () => ({ outcome: 'started' }),
    findSteamTitles: async () => [],
    // Celeste matches a Steam title; Hades does not.
    proposeCoverMatch: async (title) => (title === 'Celeste' ? { appId: 504230, name: 'Celeste' } : null),
    coverMatch: async () => null,
    setCoverMatch: async (gameId, appId) => {
      matched.push([gameId, appId]);
    },
    limitedMode: async () => ({ outcome: 'off' }),
    openSteamPrivacySettings: async () => {},
  };
  return { operations, added, matched };
}

/** The suggestions the app shows once the user chose a folder. */
async function suggestions() {
  return screen.findByRole('form', { name: 'Games in C:\\Games' });
}

describe('Adding Local Games from a folder', () => {
  it('proposes a Cover Match for each suggestion and saves it with the ticked Games', async () => {
    const user = userEvent.setup();
    const { operations, matched } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add games from a folder' }));
    const form = await suggestions();
    const [celeste, hades] = within(form).getAllByRole('listitem');
    expect(await within(celeste).findByText('Cover from Steam: Celeste')).toBeInTheDocument();
    expect(
      await within(hades).findByText('No Steam title matches. The game shows its executable’s icon.'),
    ).toBeInTheDocument();
    expect(matched).toEqual([]);

    await user.click(within(form).getByRole('checkbox', { name: 'Add Celeste' }));
    await user.click(within(form).getByRole('checkbox', { name: 'Add Hades' }));
    await user.click(within(form).getByRole('button', { name: 'Add ticked games' }));

    expect(await screen.findByText('Games added to your library: 2')).toBeInTheDocument();
    expect(matched).toEqual([['local:1', 504230]]);
  });

  it('suggests the executables in the folder with their proposed titles, none ticked', async () => {
    const user = userEvent.setup();
    const { operations, added } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add games from a folder' }));

    const form = await suggestions();
    expect(within(form).getByRole('checkbox', { name: 'Add Celeste' })).not.toBeChecked();
    expect(within(form).getByRole('checkbox', { name: 'Add Hades' })).not.toBeChecked();
    expect(within(form).getByRole('textbox', { name: `Title of ${CELESTE}` })).toHaveValue('Celeste');
    expect(within(form).getByText(HADES)).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Add ticked games' })).toBeDisabled();
    expect(added).toEqual([]);
  });

  it('marks an executable already in the Library and does not offer it', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add games from a folder' }));

    const form = await suggestions();
    const portal = within(form).getByText(PORTAL).closest('li')!;
    expect(within(portal).getByText('Portal 2')).toBeInTheDocument();
    expect(within(portal).getByText('Already in your library')).toBeInTheDocument();
    expect(within(portal).queryByRole('checkbox')).not.toBeInTheDocument();
  });

  it('adds only the ticked executables, under the titles the user corrected', async () => {
    const user = userEvent.setup();
    const { operations, added } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add games from a folder' }));
    const form = await suggestions();
    const title = within(form).getByRole('textbox', { name: `Title of ${CELESTE}` });
    await user.clear(title);
    await user.type(title, 'Celeste Classic');
    await user.click(within(form).getByRole('checkbox', { name: 'Add Celeste Classic' }));
    await user.click(within(form).getByRole('button', { name: 'Add ticked games' }));

    expect(added).toEqual([{ executable: CELESTE, title: 'Celeste Classic' }]);
    const library = await screen.findByRole('list', { name: 'Games' });
    expect(within(library).getByRole('heading', { name: 'Celeste Classic' })).toBeInTheDocument();
    expect(within(library).queryByRole('heading', { name: 'Hades' })).not.toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Games added to your library: 1');
    expect(screen.queryByRole('form', { name: 'Games in C:\\Games' })).not.toBeInTheDocument();
  });

  it('keeps the executables that could not be added ticked and says why', async () => {
    const user = userEvent.setup();
    const { operations, added } = fakeOperations({
      add: async (executable, title) =>
        executable === HADES ? { outcome: 'notFound' } : { outcome: 'added', game: localGame(1, title) },
    });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add games from a folder' }));
    const form = await suggestions();
    await user.click(within(form).getByRole('checkbox', { name: 'Add Celeste' }));
    await user.click(within(form).getByRole('checkbox', { name: 'Add Hades' }));
    await user.click(within(form).getByRole('button', { name: 'Add ticked games' }));

    expect(added.map((game) => game.executable)).toEqual([CELESTE, HADES]);
    const hades = within(form).getByText(HADES).closest('li')!;
    expect(within(hades).getByRole('alert')).toHaveTextContent('The file is no longer there.');
    expect(within(hades).getByRole('checkbox', { name: 'Add Hades' })).toBeChecked();
    const celeste = within(form).getByText(CELESTE).closest('li')!;
    expect(within(celeste).getByText('Already in your library')).toBeInTheDocument();
    expect(screen.getByText('Games added to your library: 1')).toBeInTheDocument();
  });

  it('says so when the folder holds no executables', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({
      folder: async () => ({ outcome: 'suggested', folder: FOLDER, executables: [] }),
    });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add games from a folder' }));

    expect(await screen.findByText('No executables were found in C:\\Games.')).toBeInTheDocument();
  });

  it('shows nothing when the user cancels the folder dialog, and an alert when it cannot be opened', async () => {
    const user = userEvent.setup();
    let fails = false;
    const { operations } = fakeOperations({
      folder: async () => {
        if (fails) throw new Error('folder dialog: no desktop');
        return { outcome: 'cancelled' };
      },
    });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add games from a folder' }));
    expect(screen.queryByRole('form')).not.toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();

    fails = true;
    await user.click(screen.getByRole('button', { name: 'Add games from a folder' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('The folder dialog could not be opened.');
  });

  it('speaks German', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);

    await user.click(await screen.findByRole('button', { name: 'Spiele aus einem Ordner hinzufügen' }));

    const form = await screen.findByRole('form', { name: 'Spiele in C:\\Games' });
    expect(within(form).getByRole('checkbox', { name: 'Celeste hinzufügen' })).toBeInTheDocument();
    expect(within(form).getByText('Schon in deiner Bibliothek')).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Angehakte Spiele hinzufügen' })).toBeDisabled();
  });
});
