import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LibraryFilter, LibrarySort, ManualProgress, Operations } from './operations';

function steamGame(appId: number, title: string, progress: number | null): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed: true, playtimeMinutes: 60, lastPlayed: null, progress, status: 'unplayed' };
}

function localGame(id: number, title: string, progress: number | null = null): Game {
  return { id: `local:${id}`, title, source: 'local', installed: true, playtimeMinutes: 0, lastPlayed: null, progress, status: 'unplayed' };
}

const PORTAL = steamGame(400, 'Portal', 100);
const PORTAL_2 = steamGame(620, 'Portal 2', 50);
const HALF_LIFE = steamGame(70, 'Half-Life', null);

/** Progress the most complete first, Games without Progress last, as `LibrarySort::Progress` in crates/core. */
function byProgress(a: Game, b: Game): number {
  if (a.progress !== b.progress) {
    if (a.progress === null) return 1;
    if (b.progress === null) return -1;
    return b.progress - a.progress;
  }
  return a.title.localeCompare(b.title);
}

/**
 * Fake operations over a stored Library, with the order chosen before as
 * `sort`; Local Games take Manual Progress as the core does. The filters
 * asked for are recorded.
 */
function fakeOperations(initial: Game[], sort: LibrarySort = 'name') {
  let games = initial;
  const asked: LibraryFilter[] = [];
  const operations: Operations = {
    library: async () => [...games].sort((a, b) => a.title.localeCompare(b.title)),
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
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    filterLibrary: async (filter) => {
      asked.push(filter);
      const sorted = [...games].sort((a, b) => a.title.localeCompare(b.title));
      return filter.sort === 'progress' ? sorted.sort(byProgress) : sorted;
    },
    librarySort: async () => sort,
    chooseLibrarySort: async () => {},
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    onSession: () => () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    setManualProgress: async (gameId, percent) => {
      const game = games.find((candidate) => candidate.id === gameId);
      if (!game || game.source !== 'local') return { outcome: 'notALocalGame' } satisfies ManualProgress;
      const changed = { ...game, progress: percent };
      games = games.map((candidate) => (candidate.id === gameId ? changed : candidate));
      return { outcome: 'set', game: changed } satisfies ManualProgress;
    },
  };
  return { operations, asked };
}

/** The cover card of `title` in the Library. */
async function card(title: string) {
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  const heading = await within(grid).findByRole('heading', { name: title });
  return heading.closest('li')!;
}

/** The titles of the Games in the Library, in the order shown. */
async function titles() {
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  return within(grid)
    .getAllByRole('heading')
    .map((heading) => heading.textContent);
}

describe('Progress on the cover cards', () => {
  it('shows the Progress of each Game under its Cover', async () => {
    const { operations } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="en" />);

    expect(within(await card('Portal 2')).getByText('Progress 50%')).toBeInTheDocument();
  });

  it('shows a Game without Progress with none rather than 0%', async () => {
    const { operations } = fakeOperations([HALF_LIFE, steamGame(10, 'Counter-Strike', 0)]);
    render(<App operations={operations} lang="en" />);

    expect(await card('Half-Life')).not.toHaveTextContent('Progress');
    expect(within(await card('Counter-Strike')).getByText('Progress 0%')).toBeInTheDocument();
  });

  it('shows a complete Game, 100%, in its own colour', async () => {
    const { operations } = fakeOperations([PORTAL, PORTAL_2]);
    render(<App operations={operations} lang="en" />);

    const complete = within(await card('Portal')).getByText('Progress 100%');
    const halfway = within(await card('Portal 2')).getByText('Progress 50%');

    expect(complete).toHaveAttribute('data-complete', 'true');
    expect(halfway).toHaveAttribute('data-complete', 'false');
    expect(completeColour()).toBe('var(--color-complete)');
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="de" />);

    expect(within(await card('Portal 2')).getByText('Fortschritt 50 %')).toBeInTheDocument();
  });
});

describe('Progress on the detail page', () => {
  it('shows a complete Game in the same colour as its cover card', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([PORTAL]);
    render(<App operations={operations} lang="en" />);

    await user.click(within(await card('Portal')).getByRole('button', { name: 'Details for Portal' }));
    const page = await screen.findByRole('region', { name: 'Portal' });

    expect(within(within(page).getByLabelText('About Portal')).getByText('100%')).toHaveAttribute('data-complete', 'true');
  });
});

describe('Sorting by Progress', () => {
  it('is offered with the other orders', async () => {
    const { operations } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="en" />);

    const sort = await screen.findByRole('combobox', { name: 'Sort by' });
    expect(within(sort).getByRole('option', { name: 'Progress' })).toBeInTheDocument();
  });

  it('shows the most complete Game first and the Games without Progress last', async () => {
    const user = userEvent.setup();
    const { operations, asked } = fakeOperations([HALF_LIFE, PORTAL_2, PORTAL]);
    render(<App operations={operations} lang="en" />);

    await user.selectOptions(await screen.findByRole('combobox', { name: 'Sort by' }), 'Progress');

    await expect.poll(titles).toEqual(['Portal', 'Portal 2', 'Half-Life']);
    expect(asked.at(-1)?.sort).toBe('progress');
  });

  it('stays right after the Manual Progress of a Local Game changes', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([PORTAL_2, localGame(1, 'Celeste', 10)], 'progress');
    render(<App operations={operations} lang="en" />);
    await expect.poll(titles).toEqual(['Portal 2', 'Celeste']);

    await user.click(within(await card('Celeste')).getByRole('button', { name: 'Details for Celeste' }));
    const page = await screen.findByRole('region', { name: 'Celeste' });
    const field = within(page).getByRole('textbox', { name: 'Manual Progress in percent' });
    await user.clear(field);
    await user.type(field, '80');
    await user.click(within(page).getByRole('button', { name: 'Save' }));
    expect(await within(page).findByLabelText('About Celeste')).toHaveTextContent('Progress80%');
    await user.click(within(page).getByRole('button', { name: /Back|Library/ }));

    await expect.poll(titles).toEqual(['Celeste', 'Portal 2']);
    expect(within(await card('Celeste')).getByText('Progress 80%')).toBeInTheDocument();
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="de" />);

    const sort = await screen.findByRole('combobox', { name: 'Sortieren nach' });
    expect(within(sort).getByRole('option', { name: 'Fortschritt' })).toBeInTheDocument();
  });
});

/** The colour app.css gives a complete Progress. */
function completeColour(): string | undefined {
  const css = readFileSync(join(__dirname, 'styles', 'app.css'), 'utf8');
  const rule = /\[data-complete='true'\][^{]*\{([^}]*)\}/.exec(css);
  return rule ? /(?:^|;)\s*color\s*:\s*([^;]+);/.exec(rule[1])?.[1].trim() : undefined;
}
