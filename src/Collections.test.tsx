import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Collection, CollectionNaming, Game, Operations } from './operations';

/** A promise that never settles: an operation still running. */
const pending = <T,>() => new Promise<T>(() => {});

/**
 * Fake operations whose Collections behave like the core's: names are
 * trimmed, must not be empty and must not be taken in any case, and the
 * list comes ordered by name. `calls` records what reached the core.
 */
function fakeOperations(initial: Collection[] = [], library: Game[] = []) {
  let collections = [...initial];
  let nextId = Math.max(0, ...initial.map((c) => c.id)) + 1;
  const calls = { deleted: [] as number[] };
  const name = (id: number | null, raw: string): CollectionNaming => {
    const trimmed = raw.trim();
    if (trimmed === '') return { outcome: 'emptyName' };
    if (collections.some((c) => c.id !== id && c.name.toLowerCase() === trimmed.toLowerCase())) {
      return { outcome: 'nameTaken' };
    }
    if (id === null) {
      const created = { id: nextId++, name: trimmed, gameCount: 0 };
      collections.push(created);
      return { outcome: 'named', id: created.id };
    }
    collections = collections.map((c) => (c.id === id ? { ...c, name: trimmed } : c));
    return { outcome: 'named', id };
  };
  const operations: Operations = {
    library: async () => library,
    readInstalledGames: async () => library,
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'invalid' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
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
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    onSession: () => () => {},
    filterLibrary: async () => library,
    game: async (gameId) => library.find((game) => game.id === gameId) ?? null,
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    collections: async () => [...collections].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase())),
    createCollection: async (raw) => name(null, raw),
    renameCollection: async (id, raw) => name(id, raw),
    deleteCollection: async (id) => {
      calls.deleted.push(id);
      collections = collections.filter((c) => c.id !== id);
    },
    theme: async () => 'system',
    chooseTheme: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    openLogFolder: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
  };
  return { operations, calls };
}

const BACKLOG: Collection = { id: 1, name: 'Backlog', gameCount: 12 };
const ROGUELIKES: Collection = { id: 2, name: 'Roguelikes', gameCount: 1 };

/** The Collections in the sidebar, each as its name and count. */
async function listed(lang: 'de' | 'en' = 'en') {
  const list = await screen.findByRole('list', { name: lang === 'en' ? 'Collections' : 'Sammlungen' });
  return within(list)
    .getAllByRole('listitem')
    .map((item) => item.textContent);
}

describe('Collections in the sidebar', () => {
  it('are listed with the number of Games in each', async () => {
    render(<App operations={fakeOperations([BACKLOG, ROGUELIKES]).operations} lang="en" />);

    const item = within(await screen.findByRole('list', { name: 'Collections' })).getByText('Backlog').closest('li')!;
    expect(within(item).getByText('12')).toBeInTheDocument();
    expect(within(item).getByText('12 games')).toBeInTheDocument();
    expect(await listed()).toEqual([expect.stringContaining('Backlog'), expect.stringContaining('Roguelikes')]);
  });

  it('count one Game in the singular, in German too', async () => {
    render(<App operations={fakeOperations([BACKLOG, ROGUELIKES]).operations} lang="de" />);

    expect(await screen.findByText('1 Spiel')).toBeInTheDocument();
    expect(screen.getByText('12 Spiele')).toBeInTheDocument();
  });

  it('say so when there are none yet', async () => {
    render(<App operations={fakeOperations().operations} lang="en" />);

    expect(await screen.findByText('No collections yet.')).toBeInTheDocument();
  });

  it('say so when they cannot be loaded', async () => {
    const { operations } = fakeOperations();
    operations.collections = () => Promise.reject(new Error('database: disk full'));
    render(<App operations={operations} lang="en" />);

    expect(await screen.findByRole('alert')).toHaveTextContent('Your collections could not be loaded.');
  });
});

describe('Creating a Collection', () => {
  it('adds it to the sidebar in its place by name', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations([BACKLOG, ROGUELIKES]).operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'New collection' }));
    await user.type(screen.getByRole('textbox', { name: 'Name of the new collection' }), 'Couch co-op{Enter}');

    expect(await listed()).toEqual([
      expect.stringContaining('Backlog'),
      expect.stringContaining('Couch co-op'),
      expect.stringContaining('Roguelikes'),
    ]);
    expect(screen.queryByRole('textbox', { name: 'Name of the new collection' })).not.toBeInTheDocument();
  });

  it('refuses an empty name and keeps the form open', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations([BACKLOG]).operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'New collection' }));
    await user.type(screen.getByRole('textbox', { name: 'Name of the new collection' }), '   ');
    await user.click(screen.getByRole('button', { name: 'Create' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Give the collection a name.');
    expect(screen.getByRole('textbox', { name: 'Name of the new collection' })).toBeInTheDocument();
    expect(await listed()).toHaveLength(1);
  });

  it('refuses a name another Collection has, whatever its case', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations([BACKLOG]).operations} lang="de" />);

    await user.click(await screen.findByRole('button', { name: 'Neue Sammlung' }));
    await user.type(screen.getByRole('textbox', { name: 'Name der neuen Sammlung' }), 'backlog{Enter}');

    expect(await screen.findByRole('alert')).toHaveTextContent('Es gibt schon eine Sammlung namens „backlog“.');
    expect(await listed('de')).toHaveLength(1);
  });

  it('says so when it cannot be saved', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    operations.createCollection = () => Promise.reject(new Error('database: disk full'));
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'New collection' }));
    await user.type(screen.getByRole('textbox', { name: 'Name of the new collection' }), 'Backlog{Enter}');

    expect(await screen.findByRole('alert')).toHaveTextContent('The collection could not be saved.');
  });

  it('can be cancelled', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations().operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'New collection' }));
    await user.type(screen.getByRole('textbox', { name: 'Name of the new collection' }), 'Backlog');
    await user.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
    expect(await screen.findByText('No collections yet.')).toBeInTheDocument();
  });
});

describe('Renaming a Collection', () => {
  it('shows the new name in its place by name', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations([BACKLOG, ROGUELIKES]).operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Rename Backlog' }));
    const field = screen.getByRole('textbox', { name: 'New name for Backlog' });
    expect(field).toHaveValue('Backlog');
    await user.clear(field);
    await user.type(field, 'Shelf of shame{Enter}');

    expect(await listed()).toEqual([expect.stringContaining('Roguelikes'), expect.stringContaining('Shelf of shame')]);
  });

  it('refuses a name another Collection has and keeps the old one', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations([BACKLOG, ROGUELIKES]).operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Rename Backlog' }));
    const field = screen.getByRole('textbox', { name: 'New name for Backlog' });
    await user.clear(field);
    await user.type(field, 'ROGUELIKES{Enter}');

    expect(await screen.findByRole('alert')).toHaveTextContent('A collection named “ROGUELIKES” already exists.');
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(await listed()).toEqual([expect.stringContaining('Backlog'), expect.stringContaining('Roguelikes')]);
  });

  it('refuses an empty name', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations([BACKLOG]).operations} lang="de" />);

    await user.click(await screen.findByRole('button', { name: 'Backlog umbenennen' }));
    await user.clear(screen.getByRole('textbox', { name: 'Neuer Name für Backlog' }));
    await user.click(screen.getByRole('button', { name: 'Speichern' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Gib der Sammlung einen Namen.');
  });
});

describe('Deleting a Collection', () => {
  it('asks first and deletes it only once confirmed, saying its Games stay', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations([BACKLOG, ROGUELIKES]);
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Delete Backlog' }));

    const question = screen.getByRole('alertdialog', { name: 'Delete the collection “Backlog”?' });
    expect(question).toHaveTextContent('Its games stay in your library.');
    expect(calls.deleted).toEqual([]);
    await user.click(within(question).getByRole('button', { name: 'Delete' }));

    expect(calls.deleted).toEqual([BACKLOG.id]);
    expect(await listed()).toEqual([expect.stringContaining('Roguelikes')]);
  });

  it('keeps it when the user changes their mind', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations([BACKLOG]);
    render(<App operations={operations} lang="de" />);

    await user.click(await screen.findByRole('button', { name: 'Backlog löschen' }));
    const question = screen.getByRole('alertdialog', { name: 'Sammlung „Backlog“ löschen?' });
    expect(question).toHaveTextContent('Ihre Spiele bleiben in deiner Bibliothek.');
    await user.click(within(question).getByRole('button', { name: 'Behalten' }));

    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(calls.deleted).toEqual([]);
    expect(await listed('de')).toHaveLength(1);
  });

  it('says so when it cannot be deleted, and keeps it listed', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([BACKLOG]);
    operations.deleteCollection = () => Promise.reject(new Error('database: disk full'));
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Delete Backlog' }));
    await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Delete' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The collection could not be deleted.');
    expect(await listed()).toHaveLength(1);
  });

  it('removes no Game from the Library', async () => {
    const user = userEvent.setup();
    const portal: Game = { id: 'steam:620', title: 'Portal 2', source: 'steam', installed: true, playtimeMinutes: null, lastPlayed: null, status: 'unplayed' };
    render(<App operations={fakeOperations([BACKLOG], [portal]).operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Delete Backlog' }));
    await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Delete' }));

    expect(await screen.findByText('No collections yet.')).toBeInTheDocument();
    expect(within(screen.getByRole('list', { name: 'Games' })).getByText('Portal 2')).toBeInTheDocument();
  });
});

describe('While Collections are still loading', () => {
  it('nothing is listed yet and nothing fails', async () => {
    const { operations } = fakeOperations();
    operations.collections = pending;
    render(<App operations={operations} lang="en" />);

    expect(await screen.findByRole('button', { name: 'New collection' })).toBeInTheDocument();
    expect(screen.queryByText('No collections yet.')).not.toBeInTheDocument();
  });
});
