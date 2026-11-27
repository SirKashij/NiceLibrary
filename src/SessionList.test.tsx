import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import type { Game, Operations, Session, SessionEvent } from './operations';

/** 2026-11-27 10:45:00 UTC, in seconds since the Unix epoch. */
const THIS_MORNING = 1_795_776_300;
/** 2026-11-20 20:15:00 UTC. */
const LAST_WEEK = 1_795_205_700;

function localGame(id: number, title: string, details: Partial<Game> = {}): Game {
  return { id: `local:${id}`, title, source: 'local', installed: true, playtimeMinutes: 0, lastPlayed: null, status: 'unplayed', ...details };
}

const CELESTE = localGame(1, 'Celeste', { playtimeMinutes: 115, lastPlayed: THIS_MORNING });

/**
 * Fake operations over one Library whose Sessions the test sets per Game,
 * and a tracker whose Session events the test sends. Calls to `sessions`
 * are recorded.
 */
function fakeOperations(games: Game[], initial: Record<string, Session[]> = {}) {
  let sessions = initial;
  const asked: string[] = [];
  const listeners = new Set<(event: SessionEvent) => void>();
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
    filterLibrary: async () => games,
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
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
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    onSession: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    sessions: async (gameId) => {
      asked.push(gameId);
      return sessions[gameId] ?? [];
    },
  };
  const send = (event: SessionEvent) => act(() => listeners.forEach((listener) => listener(event)));
  return { operations, asked, send, change: (next: Record<string, Session[]>) => (sessions = next) };
}

async function openDetail(title: string, user = userEvent.setup()) {
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: new RegExp(`(Details for|Details zu) ${title}`) }));
  return screen.findByRole('region', { name: title });
}

afterEach(() => {
  vi.useRealTimers();
});

describe('Sessions on the detail page', () => {
  it('lists the Sessions newest first, each with its date and its length in hours and minutes', async () => {
    const { operations } = fakeOperations([CELESTE], {
      'local:1': [
        { startedAt: THIS_MORNING, seconds: 85 * 60 + 30, running: false },
        { startedAt: LAST_WEEK, seconds: 30 * 60, running: false },
      ],
    });
    render(<App operations={operations} lang="en" />);

    const page = await openDetail('Celeste');

    const list = await within(page).findByRole('list', { name: 'Sessions of Celeste' });
    const items = within(list).getAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent('Nov 27, 2026');
    expect(items[0]).toHaveTextContent('1 h 25 min');
    expect(items[1]).toHaveTextContent('Nov 20, 2026');
    expect(items[1]).toHaveTextContent('30 min');
    expect(items[1]).not.toHaveTextContent(/\d h/);
  });

  it('writes the lengths in German in the German interface', async () => {
    const { operations } = fakeOperations([CELESTE], {
      'local:1': [
        { startedAt: THIS_MORNING, seconds: 2 * 3600, running: false },
        { startedAt: LAST_WEEK, seconds: 3600 + 5 * 60, running: false },
        { startedAt: LAST_WEEK - 86_400, seconds: 20, running: false },
      ],
    });
    render(<App operations={operations} lang="de" />);

    const page = await openDetail('Celeste');

    const list = await within(page).findByRole('list', { name: 'Spielsitzungen von Celeste' });
    const items = within(list).getAllByRole('listitem');
    expect(items[0]).toHaveTextContent('27.11.2026');
    expect(items[0]).toHaveTextContent('2 Std. 0 Min.');
    expect(items[1]).toHaveTextContent('1 Std. 5 Min.');
    expect(items[2]).toHaveTextContent('unter 1 Min.');
  });

  it('explains that a Game without Sessions has none yet', async () => {
    const { operations } = fakeOperations([localGame(2, 'Hades')]);
    render(<App operations={operations} lang="en" />);

    const page = await openDetail('Hades');

    expect(
      await within(page).findByText('No sessions yet. A session is recorded whenever Hades runs, however you start it.'),
    ).toBeInTheDocument();
    expect(within(page).queryByRole('list', { name: 'Sessions of Hades' })).not.toBeInTheDocument();
  });

  it('explains it in German too', async () => {
    const { operations } = fakeOperations([localGame(2, 'Hades')]);
    render(<App operations={operations} lang="de" />);

    const page = await openDetail('Hades');

    expect(
      await within(page).findByText(
        'Noch keine Spielsitzungen. Eine Spielsitzung wird aufgezeichnet, sobald Hades läuft, egal wie du es startest.',
      ),
    ).toBeInTheDocument();
  });

  it('marks a Session that is running as running', async () => {
    const { operations } = fakeOperations([CELESTE], {
      'local:1': [
        { startedAt: THIS_MORNING, seconds: 10 * 60, running: true },
        { startedAt: LAST_WEEK, seconds: 30 * 60, running: false },
      ],
    });
    render(<App operations={operations} lang="en" />);

    const page = await openDetail('Celeste');

    const items = within(await within(page).findByRole('list', { name: 'Sessions of Celeste' })).getAllByRole('listitem');
    expect(items[0]).toHaveTextContent('Running');
    expect(items[0]).toHaveTextContent('10 min');
    expect(items[1]).not.toHaveTextContent('Running');
  });

  it('lists the Sessions again when a Session begins or ends', async () => {
    const { operations, send, change } = fakeOperations([CELESTE]);
    render(<App operations={operations} lang="en" />);
    const page = await openDetail('Celeste');
    await within(page).findByText(/No sessions yet/);

    change({ 'local:1': [{ startedAt: THIS_MORNING, seconds: 0, running: true }] });
    send({ state: 'started', gameId: 'local:1' });
    const list = await within(page).findByRole('list', { name: 'Sessions of Celeste' });
    expect(await within(list).findByText('Running')).toBeInTheDocument();

    change({ 'local:1': [{ startedAt: THIS_MORNING, seconds: 45 * 60, running: false }] });
    send({ state: 'ended', gameId: 'local:1' });
    expect(await within(list).findByText('45 min')).toBeInTheDocument();
    expect(within(list).queryByText('Running')).not.toBeInTheDocument();
  });

  it('keeps the length of a running Session up to date while the page is open', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { operations, change } = fakeOperations([CELESTE], {
      'local:1': [{ startedAt: THIS_MORNING, seconds: 5 * 60, running: true }],
    });
    render(<App operations={operations} lang="en" />);
    const page = await openDetail('Celeste', userEvent.setup({ advanceTimers: vi.advanceTimersByTime }));
    const list = await within(page).findByRole('list', { name: 'Sessions of Celeste' });
    expect(within(list).getByRole('listitem')).toHaveTextContent('5 min');

    change({ 'local:1': [{ startedAt: THIS_MORNING, seconds: 6 * 60, running: true }] });
    await act(() => vi.advanceTimersByTimeAsync(60_000));

    expect(within(list).getByRole('listitem')).toHaveTextContent('6 min');
  });

  it('says so when the Sessions cannot be read', async () => {
    const { operations } = fakeOperations([CELESTE]);
    operations.sessions = async () => {
      throw new Error('database: locked');
    };
    render(<App operations={operations} lang="en" />);

    const page = await openDetail('Celeste');

    expect(await within(page).findByRole('alert')).toHaveTextContent('The sessions of Celeste could not be read.');
  });
});
