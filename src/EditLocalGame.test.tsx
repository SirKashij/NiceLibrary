import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { EditLocalGame, ExecutableChoice, Game, LocalGame, Operations } from './operations';

const CELESTE: Game = {
  id: 'local:1',
  title: 'Celeste',
  source: 'local',
  installed: true,
  playtimeMinutes: 0,
  lastPlayed: null,
  status: 'unplayed',
  progress: null,
};

const PORTAL_2: Game = {
  id: 'steam:620',
  title: 'Portal 2',
  source: 'steam',
  installed: true,
  playtimeMinutes: 1_260,
  lastPlayed: null,
  status: 'unplayed',
  progress: null,
};

const CELESTE_EXE = 'C:\\Games\\Celeste\\Celeste.exe';

/**
 * Fake operations over a stored Library with one Local Game, Celeste, and one
 * Steam Game. Editing and removing change the Library as the core would.
 * Calls are recorded.
 */
function fakeOperations(
  options: {
    edit?: (gameId: string, title: string, executable: string) => Promise<EditLocalGame>;
    choose?: () => Promise<ExecutableChoice>;
    remove?: () => Promise<void>;
  } = {},
) {
  let games = [CELESTE, PORTAL_2];
  const executables: Record<string, string> = { [CELESTE.id]: CELESTE_EXE };
  const calls = { edited: [] as [string, string, string][], removed: [] as string[], chosenFor: [] as string[] };
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
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
    theme: async () => 'system',
    chooseTheme: async () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    onAutomaticSync: () => () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    filterLibrary: async () => games,
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    localGame: async (gameId): Promise<LocalGame | null> => {
      const game = games.find((game) => game.id === gameId && game.source === 'local');
      return game ? { id: game.id, title: game.title, executable: executables[game.id] } : null;
    },
    chooseExecutableFor: (gameId) => {
      calls.chosenFor.push(gameId);
      return options.choose ? options.choose() : Promise.resolve({ outcome: 'cancelled' });
    },
    editLocalGame: async (gameId, title, executable) => {
      calls.edited.push([gameId, title, executable]);
      if (options.edit) return options.edit(gameId, title, executable);
      const game = { ...games.find((game) => game.id === gameId)!, title: title.trim() };
      games = games.map((other) => (other.id === gameId ? game : other));
      executables[gameId] = executable;
      return { outcome: 'edited', game };
    },
    removeLocalGame: async (gameId) => {
      calls.removed.push(gameId);
      if (options.remove) return options.remove();
      games = games.filter((game) => game.id !== gameId);
    },
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    openLogFolder: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    startsWithWindows: async () => false,
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
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
  };
  return { operations, calls };
}

async function open(title: string, lang: 'en' | 'de' = 'en') {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: lang === 'en' ? 'Games' : 'Spiele' });
  await user.click(within(grid).getByRole('button', { name: lang === 'en' ? `Details for ${title}` : `Details zu ${title}` }));
  return { user, page: await screen.findByRole('region', { name: title }) };
}

describe('Editing a Local Game', () => {
  it('changes its title from the detail page, which then shows the new title', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Edit Celeste' }));
    const form = await screen.findByRole('form', { name: 'Edit Celeste' });
    expect(within(form).getByText(CELESTE_EXE)).toBeInTheDocument();
    const title = within(form).getByLabelText('Title');
    expect(title).toHaveValue('Celeste');
    await user.type(title, ' Classic');
    await user.click(within(form).getByRole('button', { name: 'Save' }));

    expect(calls.edited).toEqual([['local:1', 'Celeste Classic', CELESTE_EXE]]);
    expect(await screen.findByRole('heading', { level: 1, name: 'Celeste Classic' })).toBeInTheDocument();
    expect(screen.queryByRole('form', { name: /Edit/ })).not.toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Your changes were saved.');
  });

  it('replaces its executable with one chosen in the file dialog', async () => {
    const other = 'D:\\Games\\Celeste\\Celeste.exe';
    const { operations, calls } = fakeOperations({
      choose: async () => ({ outcome: 'chosen', executable: other, title: 'Celeste' }),
    });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Edit Celeste' }));
    const form = await screen.findByRole('form', { name: 'Edit Celeste' });
    await user.click(within(form).getByRole('button', { name: 'Choose another file' }));
    expect(await within(form).findByText(other)).toBeInTheDocument();
    await user.click(within(form).getByRole('button', { name: 'Save' }));

    expect(calls.chosenFor).toEqual(['local:1']);
    expect(calls.edited).toEqual([['local:1', 'Celeste', other]]);
  });

  it('names the Game a chosen executable already belongs to and keeps the old one', async () => {
    const { operations } = fakeOperations({ choose: async () => ({ outcome: 'alreadyAdded', title: 'Zork' }) });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Edit Celeste' }));
    const form = await screen.findByRole('form', { name: 'Edit Celeste' });
    await user.click(within(form).getByRole('button', { name: 'Choose another file' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('This file already belongs to Zork in your library.');
    expect(within(form).getByText(CELESTE_EXE)).toBeInTheDocument();
  });

  it('says why the core refused the changes and keeps the form open', async () => {
    const { operations } = fakeOperations({ edit: async () => ({ outcome: 'noTitle' }) });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Edit Celeste' }));
    const form = await screen.findByRole('form', { name: 'Edit Celeste' });
    await user.clear(within(form).getByLabelText('Title'));
    await user.click(within(form).getByRole('button', { name: 'Save' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Enter a title for the game.');
    expect(screen.getByRole('form', { name: 'Edit Celeste' })).toBeInTheDocument();
  });

  it('cancelling changes nothing', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Edit Celeste' }));
    const form = await screen.findByRole('form', { name: 'Edit Celeste' });
    await user.type(within(form).getByLabelText('Title'), ' typo');
    await user.click(within(form).getByRole('button', { name: 'Cancel' }));

    expect(calls.edited).toEqual([]);
    expect(screen.queryByRole('form', { name: 'Edit Celeste' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1, name: 'Celeste' })).toBeInTheDocument();
  });
});

describe('Removing a Local Game', () => {
  it('asks first, says the files stay, and only then removes it and returns to the Library', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Remove Celeste' }));
    const question = screen.getByRole('alertdialog', { name: 'Remove Celeste from your library?' });
    expect(question).toHaveAccessibleDescription(
      'Its Collection memberships and everything else NiceLibrary keeps about it are deleted. The game’s files stay on your computer.',
    );
    expect(calls.removed).toEqual([]);
    await user.click(within(question).getByRole('button', { name: 'Remove' }));

    expect(calls.removed).toEqual(['local:1']);
    const grid = await screen.findByRole('list', { name: 'Games' });
    expect(within(grid).queryByText('Celeste')).not.toBeInTheDocument();
    expect(within(grid).getByText('Portal 2')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Celeste was removed from your library. Its files are still on your computer.');
  });

  it('keeping it changes nothing', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Remove Celeste' }));
    await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Keep' }));

    expect(calls.removed).toEqual([]);
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1, name: 'Celeste' })).toBeInTheDocument();
  });

  it('says so when it cannot be removed, and stays on the detail page', async () => {
    const { operations } = fakeOperations({ remove: () => Promise.reject(new Error('database: disk full')) });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Remove Celeste' }));
    await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Remove' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Celeste could not be removed.');
    expect(screen.getByRole('heading', { level: 1, name: 'Celeste' })).toBeInTheDocument();
  });
});

describe('A Steam Game', () => {
  it('offers neither editing nor removing', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { page } = await open('Portal 2');

    expect(within(page).getByRole('button', { name: 'Play Portal 2' })).toBeInTheDocument();
    expect(within(page).queryByRole('button', { name: /Edit/ })).not.toBeInTheDocument();
    expect(within(page).queryByRole('button', { name: /Remove/ })).not.toBeInTheDocument();
  });
});

describe('In German', () => {
  it('edits and removes in German', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);
    const { user, page } = await open('Celeste', 'de');

    await user.click(within(page).getByRole('button', { name: 'Celeste bearbeiten' }));
    const form = await screen.findByRole('form', { name: 'Celeste bearbeiten' });
    expect(within(form).getByLabelText('Titel')).toHaveValue('Celeste');
    expect(within(form).getByRole('button', { name: 'Andere Datei wählen' })).toBeInTheDocument();
    await user.click(within(form).getByRole('button', { name: 'Abbrechen' }));

    await user.click(within(page).getByRole('button', { name: 'Celeste entfernen' }));
    const question = screen.getByRole('alertdialog', { name: 'Celeste aus deiner Bibliothek entfernen?' });
    expect(question).toHaveAccessibleDescription(
      'Seine Mitgliedschaften in Sammlungen und alles andere, was NiceLibrary darüber speichert, werden gelöscht. Die Dateien des Spiels bleiben auf deinem Computer.',
    );
    await user.click(within(question).getByRole('button', { name: 'Entfernen' }));

    expect(await screen.findByRole('status')).toHaveTextContent(
      'Celeste wurde aus deiner Bibliothek entfernt. Seine Dateien sind noch auf deinem Computer.',
    );
  });
});
