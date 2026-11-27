import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, Operations, SessionEvent } from './operations';

/** 2026-11-27 10:45:00 UTC, in seconds since the Unix epoch. */
const THIS_MORNING = 1_795_776_300;

function localGame(id: number, title: string, details: Partial<Game> = {}): Game {
  return { id: `local:${id}`, title, source: 'local', installed: true, playtimeMinutes: 0, lastPlayed: null, status: 'unplayed', ...details };
}

const CELESTE = localGame(1, 'Celeste');

/**
 * Fake operations over a stored Library the test can change, and a tracker
 * whose Session events the test sends.
 */
function fakeOperations(initial: Game[]) {
  let games = initial;
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
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
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
  };
  const send = (event: SessionEvent) => act(() => listeners.forEach((listener) => listener(event)));
  return { operations, send, listening: () => listeners.size, change: (next: Game[]) => (games = next) };
}

describe('Sessions of Local Games', () => {
  it('shows the new Playtime and Last Played in the Library once a Session ends', async () => {
    const { operations, send, change } = fakeOperations([CELESTE]);
    render(<App operations={operations} lang="en" />);
    const grid = await screen.findByRole('list', { name: 'Games' });
    expect(within(grid).getByText('Never played')).toBeInTheDocument();

    change([{ ...CELESTE, playtimeMinutes: 45, lastPlayed: THIS_MORNING }]);
    await send({ state: 'ended', gameId: CELESTE.id });

    expect(await within(grid).findByText('45 min played · last Nov 27, 2026')).toBeInTheDocument();
  });

  it('shows it in German as well', async () => {
    const { operations, send, change } = fakeOperations([CELESTE]);
    render(<App operations={operations} lang="de" />);
    const grid = await screen.findByRole('list', { name: 'Spiele' });

    change([{ ...CELESTE, playtimeMinutes: 90, lastPlayed: THIS_MORNING }]);
    await send({ state: 'ended', gameId: CELESTE.id });

    expect(await within(grid).findByText(/1,5 Std\..*27\.11\.2026/)).toBeInTheDocument();
  });

  it('updates the open detail page when a Session of its Game begins', async () => {
    const user = userEvent.setup();
    const { operations, send, change } = fakeOperations([CELESTE]);
    render(<App operations={operations} lang="en" />);
    const grid = await screen.findByRole('list', { name: 'Games' });
    await user.click(within(grid).getByRole('button', { name: 'Details for Celeste' }));
    const facts = within(await screen.findByRole('region', { name: 'Celeste' })).getByLabelText('About Celeste');
    expect(facts).toHaveTextContent('Last playedNever');

    change([{ ...CELESTE, lastPlayed: THIS_MORNING }]);
    await send({ state: 'started', gameId: CELESTE.id });

    await expect.poll(() => facts.textContent).toContain('Last playedNov 27, 2026');
  });

  it('stops listening when the app closes', async () => {
    const { operations, listening } = fakeOperations([CELESTE]);
    const { unmount } = render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });
    expect(listening()).toBe(1);

    unmount();

    expect(listening()).toBe(0);
  });
});
