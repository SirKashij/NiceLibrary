import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LibraryFilter, Operations } from './operations';

function steamGame(appId: number, title: string, installed: boolean): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed, playtimeMinutes: null, lastPlayed: null };
}

const LIBRARY = [steamGame(1145360, 'Hades', true), steamGame(220, 'Half-Life 2', false), steamGame(620, 'Portal 2', true)];

/**
 * Fake operations over LIBRARY. `filterLibrary` records each filter it is
 * asked for and answers like the core does.
 */
function fakeOperations() {
  const asked: LibraryFilter[] = [];
  const operations: Operations = {
    library: async () => LIBRARY,
    readInstalledGames: async () => LIBRARY,
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
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    filterLibrary: async (filter) => {
      asked.push(filter);
      return LIBRARY.filter(
        (game) =>
          (filter.source === null || game.source === filter.source) &&
          (!filter.installedOnly || game.installed) &&
          game.title.toLowerCase().includes(filter.search.toLowerCase()),
      );
    },
    game: async (gameId) => LIBRARY.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
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

    expect(asked.at(-1)).toEqual({ search: '', source: 'steam', installedOnly: false });
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

    expect(asked.at(-1)).toEqual({ search: 'ha', source: 'steam', installedOnly: true });
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
    expect(asked.at(-1)).toEqual({ search: 'half', source: null, installedOnly: true });
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
