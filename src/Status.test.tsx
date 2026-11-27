import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LibraryFilter, Operations, Status } from './operations';

function steamGame(appId: number, title: string, installed: boolean, status: Status = 'unplayed'): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed, playtimeMinutes: null, lastPlayed: null, status };
}

/**
 * Fake operations over a Library of Hades (Playing), Half-Life 2 and Portal 2.
 * `setStatus` changes the Library like the core does and records each call;
 * `filterLibrary` records each filter and answers like the core does.
 */
function fakeOperations(options: { setStatus?: Operations['setStatus'] } = {}) {
  let games = [
    steamGame(1145360, 'Hades', true, 'playing'),
    steamGame(220, 'Half-Life 2', false),
    steamGame(620, 'Portal 2', true),
  ];
  const calls = { setStatus: [] as [string, Status][], filters: [] as LibraryFilter[] };
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
    onAutomaticSync: () => () => {},
    filterLibrary: async (filter) => {
      calls.filters.push(filter);
      return games.filter(
        (game) =>
          (filter.source === null || game.source === filter.source) &&
          (!filter.installedOnly || game.installed) &&
          (filter.status === null || game.status === filter.status) &&
          game.title.toLowerCase().includes(filter.search.toLowerCase()),
      );
    },
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    openLogFolder: async () => {},
    limitedMode: async () => ({ outcome: 'off' }),
    openSteamPrivacySettings: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
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
    setStatus:
      options.setStatus ??
      (async (gameId, status) => {
        calls.setStatus.push([gameId, status]);
        games = games.map((game) => (game.id === gameId ? { ...game, status } : game));
        const game = games.find((game) => game.id === gameId);
        if (!game) throw new Error(`no such game in the library: ${gameId}`);
        return game;
      }),
  };
  return { operations, calls };
}

/** The titles the Library shows, once it shows `count` Games. */
async function shownTitles(count: number) {
  const list = await screen.findByRole('list', { name: /^(Games|Spiele)$/ });
  await expect.poll(() => within(list).queryAllByRole('listitem').length).toBe(count);
  return within(list)
    .getAllByRole('heading')
    .map((heading) => heading.textContent);
}

/** The Game in the Library titled `title`. */
async function inLibrary(title: string) {
  const list = await screen.findByRole('list', { name: /^(Games|Spiele)$/ });
  const heading = await within(list).findByRole('heading', { name: title });
  return heading.closest('li') as HTMLElement;
}

async function openDetailPage(title: string) {
  const user = userEvent.setup();
  await user.click(within(await inLibrary(title)).getByRole('button', { name: `Details for ${title}` }));
  return { user, page: await screen.findByRole('region', { name: title }) };
}

describe('The Status of a Game', () => {
  it('shows in the Library for every Game', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    expect(await inLibrary('Hades')).toHaveTextContent('Playing');
    expect(await inLibrary('Portal 2')).toHaveTextContent('Unplayed');
  });

  it('shows on the detail page as the chosen one of the four', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Hades');

    const status = within(page).getByRole('group', { name: 'Status of Hades' });
    expect(within(status).getAllByRole('button').map((button) => button.textContent)).toEqual([
      'Unplayed',
      'Playing',
      'Completed',
      'Dropped',
    ]);
    expect(within(status).getByRole('button', { name: 'Playing' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(status).getByRole('button', { name: 'Unplayed' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('is set on the detail page and then shows there and in the Library', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetailPage('Portal 2');

    const status = within(page).getByRole('group', { name: 'Status of Portal 2' });
    await user.click(within(status).getByRole('button', { name: 'Completed' }));

    expect(calls.setStatus).toEqual([['steam:620', 'completed']]);
    await expect
      .poll(() => within(status).getByRole('button', { name: 'Completed' }).getAttribute('aria-pressed'))
      .toBe('true');
    expect(within(status).getByRole('button', { name: 'Unplayed' })).toHaveAttribute('aria-pressed', 'false');

    await user.click(screen.getByRole('button', { name: 'Back to the library' }));
    await expect.poll(async () => (await inLibrary('Portal 2')).textContent).toContain('Completed');
  });

  it('says so when it could not be set, and keeps the Status shown before', async () => {
    const { operations } = fakeOperations({ setStatus: () => Promise.reject(new Error('database: disk I/O error')) });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetailPage('Hades');

    const status = within(page).getByRole('group', { name: 'Status of Hades' });
    await user.click(within(status).getByRole('button', { name: 'Dropped' }));

    expect(await within(page).findByRole('alert')).toHaveTextContent('The status of Hades could not be changed.');
    expect(within(status).getByRole('button', { name: 'Playing' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('is in German for a German interface', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);

    expect(await inLibrary('Hades')).toHaveTextContent('Wird gespielt');
    expect(screen.getByRole('combobox', { name: 'Status' })).toHaveDisplayValue('Jeder Status');
    await user.click(within(await inLibrary('Hades')).getByRole('button', { name: 'Details zu Hades' }));
    const status = within(await screen.findByRole('region', { name: 'Hades' })).getByRole('group', {
      name: 'Status von Hades',
    });
    expect(within(status).getAllByRole('button').map((button) => button.textContent)).toEqual([
      'Ungespielt',
      'Wird gespielt',
      'Durchgespielt',
      'Abgebrochen',
    ]);
  });
});

describe('Filtering the Library by Status', () => {
  it('asks the core for the Games with the chosen Status', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);
    expect(screen.getByRole('combobox', { name: 'Status' })).toHaveDisplayValue('Any status');

    await user.selectOptions(screen.getByRole('combobox', { name: 'Status' }), 'Playing');

    expect(calls.filters.at(-1)).toEqual({ search: '', source: null, installedOnly: false, status: 'playing', sort: 'name' });
    expect(await shownTitles(1)).toEqual(['Hades']);
  });

  it('combines with the other filters and the search', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.selectOptions(screen.getByRole('combobox', { name: 'Status' }), 'Unplayed');
    expect(await shownTitles(2)).toEqual(['Half-Life 2', 'Portal 2']);
    await user.click(screen.getByRole('checkbox', { name: 'Installed only' }));
    await user.type(screen.getByRole('searchbox', { name: 'Search the library' }), 'portal');

    expect(calls.filters.at(-1)).toEqual({ search: 'portal', source: null, installedOnly: true, status: 'unplayed', sort: 'name' });
    expect(await shownTitles(1)).toEqual(['Portal 2']);
  });

  it('shows the empty state when no Game has the Status, whose action clears it too', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);

    await user.selectOptions(screen.getByRole('combobox', { name: 'Status' }), 'Dropped');
    expect(await screen.findByText('No games match these filters.')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Reset filters' }));

    expect(await shownTitles(3)).toEqual(['Hades', 'Half-Life 2', 'Portal 2']);
    expect(screen.getByRole('combobox', { name: 'Status' })).toHaveDisplayValue('Any status');
  });

  it('leaves out a Game whose Status was changed on its detail page', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    await shownTitles(3);
    await userEvent.setup().selectOptions(screen.getByRole('combobox', { name: 'Status' }), 'Playing');
    expect(await shownTitles(1)).toEqual(['Hades']);

    const { user, page } = await openDetailPage('Hades');
    await user.click(
      within(within(page).getByRole('group', { name: 'Status of Hades' })).getByRole('button', { name: 'Completed' }),
    );
    await expect
      .poll(() => within(page).getByRole('button', { name: 'Completed' }).getAttribute('aria-pressed'))
      .toBe('true');
    await user.click(screen.getByRole('button', { name: 'Back to the library' }));

    expect(await screen.findByText('No games match these filters.')).toBeInTheDocument();
  });
});
