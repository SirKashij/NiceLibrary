import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { AutomaticSync, Game, Operations, SteamWebApiKeyCheck, SyncOutcome } from './operations';

/** 2026-11-25 12:00 UTC, in seconds since the Unix epoch. */
const NOW = 1_795_608_000;
/** 2026-11-20 20:15 UTC. */
const LAST_WEEK = 1_795_205_700;

/** A promise that never settles: an operation still running. */
const pending = <T,>() => new Promise<T>(() => {});

function steamGame(appId: number, title: string, installed: boolean, minutes: number | null, lastPlayed: number | null): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed, playtimeMinutes: minutes, lastPlayed, progress: null, status: 'unplayed', favorite: false };
}

/**
 * Fake operations that behave like the core: a Sync answers `outcome`, and
 * only a successful one replaces the stored Library with `synced` and
 * remembers when. Storing a key answers `keyCheck`; a stored key starts an
 * automatic Sync right away, which `calls.afterKeyStored` holds.
 */
function fakeOperations({
  before = [] as Game[],
  synced = [] as Game[],
  lastSync = null as number | null,
  outcome = { outcome: 'synced', at: NOW } as SyncOutcome,
  keyCheck = { outcome: 'stored' } as SteamWebApiKeyCheck,
  keyStored = false,
} = {}) {
  const calls: { syncs: number; afterKeyStored?: ReturnType<typeof automaticSync> } = { syncs: 0 };
  let library = before;
  let last = lastSync;
  const listeners = new Set<(event: AutomaticSync) => void>();
  /** The core runs an automatic Sync: it starts, and `finish` ends it with `outcome`, as `sync` would. */
  const automaticSync = () => {
    listeners.forEach((listener) => listener({ state: 'started' }));
    return {
      finish(answer: SyncOutcome = outcome) {
        if (answer.outcome === 'synced') {
          library = synced;
          last = answer.at;
        }
        listeners.forEach((listener) => listener({ state: 'finished', ...answer }));
      },
      fail() {
        listeners.forEach((listener) => listener({ state: 'failed', reason: 'database: disk full' }));
      },
    };
  };
  const operations: Operations = {
    library: async () => library,
    readInstalledGames: async () => library,
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => keyStored,
    storeSteamWebApiKey: async () => {
      // As in the app, the Sync is heard only after the answer: it runs on a thread of its own.
      if (keyCheck.outcome === 'stored') setTimeout(() => (calls.afterKeyStored = automaticSync()));
      return keyCheck;
    },
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    openLogFolder: async () => {},
    sync: async () => {
      calls.syncs += 1;
      if (outcome.outcome === 'synced') {
        library = synced;
        last = outcome.at;
      }
      return outcome;
    },
    lastSync: async () => last,
    theme: async () => 'system',
    chooseTheme: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    filterLibrary: async () => [],
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    onAutomaticSync: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    game: async (gameId) => library.find((game) => game.id === gameId) ?? null,
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    onSession: () => () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
  };
  return { operations, calls, automaticSync, listeners };
}

function syncRegion(name = 'Sync') {
  return within(screen.getByRole('complementary')).getByRole('region', { name });
}

function gameTitles() {
  return within(screen.getByRole('list', { name: 'Games' }))
    .getAllByRole('listitem')
    .map((item) => within(item).getByRole('heading').textContent);
}

describe('The last Sync in the sidebar', () => {
  it('says when the last successful Sync happened', async () => {
    render(<App operations={fakeOperations({ lastSync: LAST_WEEK }).operations} lang="en" />);

    expect(await within(syncRegion()).findByText('Last synced Nov 20, 2026, 8:15 PM')).toBeInTheDocument();
  });

  it('says when there has been no Sync yet', async () => {
    render(<App operations={fakeOperations().operations} lang="en" />);

    expect(await within(syncRegion()).findByText('Not synced yet')).toBeInTheDocument();
  });

  it('speaks German', async () => {
    render(<App operations={fakeOperations({ lastSync: LAST_WEEK }).operations} lang="de" />);

    const region = syncRegion('Synchronisierung');
    expect(await within(region).findByText('Zuletzt synchronisiert am 20.11.2026, 20:15')).toBeInTheDocument();
    expect(within(region).getByRole('button', { name: 'Jetzt synchronisieren' })).toBeInTheDocument();
  });
});

describe('Sync now', () => {
  it('brings every owned Steam Game into the Library and shows the new time', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations({
      before: [steamGame(620, 'Portal 2', true, null, null)],
      synced: [steamGame(400, 'Portal', false, 95, LAST_WEEK), steamGame(620, 'Portal 2', true, 1234, NOW - 3600)],
      lastSync: LAST_WEEK,
    });
    render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });

    await user.click(within(syncRegion()).getByRole('button', { name: 'Sync now' }));

    expect(await within(syncRegion()).findByText('Last synced Nov 25, 2026, 12:00 PM')).toBeInTheDocument();
    expect(calls.syncs).toBe(1);
    await screen.findByText('Portal');
    expect(gameTitles()).toEqual(['Portal', 'Portal 2']);
    const [portal] = within(screen.getByRole('list', { name: 'Games' })).getAllByRole('listitem');
    expect(within(portal).getByText('Not installed')).toBeInTheDocument();
  });

  it('cannot be started twice at once and says that it is running', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    let syncs = 0;
    render(
      <App
        operations={{
          ...operations,
          sync: () => {
            syncs += 1;
            return pending();
          },
        }}
        lang="en"
      />,
    );

    await user.click(within(syncRegion()).getByRole('button', { name: 'Sync now' }));

    const button = within(syncRegion()).getByRole('button', { name: 'Syncing…' });
    expect(button).toBeDisabled();
    await user.click(button);
    expect(syncs).toBe(1);
  });

  it.each([
    ['noKey', 'Add your Steam Web API key in the settings to sync your Steam library.'],
    ['noSteamAccount', 'Choose your Steam account in the settings to sync your Steam library.'],
    ['keyRejected', 'Steam rejected your Steam Web API key. Replace it in the settings. Nothing was changed.'],
    [
      'gameDetailsPrivate',
      'Steam does not share your games because your game details are private. Set them to public in your Steam privacy settings. Nothing was changed.',
    ],
    ['steamUnreachable', 'Steam could not be reached. Nothing was changed. Try again later.'],
  ] as const)('says what went wrong when the Sync fails with %s, and changes nothing', async (outcome, message) => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({
      before: [steamGame(620, 'Portal 2', true, 1234, LAST_WEEK)],
      lastSync: LAST_WEEK,
      outcome: { outcome },
    });
    render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });

    await user.click(within(syncRegion()).getByRole('button', { name: 'Sync now' }));

    expect(await within(syncRegion()).findByRole('alert')).toHaveTextContent(message);
    expect(within(syncRegion()).getByText('Last synced Nov 20, 2026, 8:15 PM')).toBeInTheDocument();
    expect(gameTitles()).toEqual(['Portal 2']);
  });

  it('says that the Sync failed when the core reports an error', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ lastSync: LAST_WEEK });
    render(<App operations={{ ...operations, sync: () => Promise.reject(new Error('database: disk full')) }} lang="en" />);

    await user.click(within(syncRegion()).getByRole('button', { name: 'Sync now' }));

    expect(await within(syncRegion()).findByRole('alert')).toHaveTextContent('The sync failed. Nothing was changed.');
  });

  it('clears the last problem once a Sync succeeds', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    let answer: SyncOutcome = { outcome: 'steamUnreachable' };
    render(<App operations={{ ...operations, sync: async () => answer }} lang="en" />);
    const button = within(syncRegion()).getByRole('button', { name: 'Sync now' });
    await user.click(button);
    await within(syncRegion()).findByRole('alert');

    answer = { outcome: 'synced', at: NOW };
    await user.click(within(syncRegion()).getByRole('button', { name: 'Sync now' }));

    expect(await within(syncRegion()).findByText('Last synced Nov 25, 2026, 12:00 PM')).toBeInTheDocument();
    expect(within(syncRegion()).queryByRole('alert')).not.toBeInTheDocument();
  });
});

describe('An automatic Sync', () => {
  it('says that it is running, then brings the Library and the new time', async () => {
    const { operations, automaticSync } = fakeOperations({
      before: [steamGame(620, 'Portal 2', true, null, null)],
      synced: [steamGame(400, 'Portal', false, 95, LAST_WEEK), steamGame(620, 'Portal 2', true, 1234, NOW - 3600)],
      lastSync: LAST_WEEK,
    });
    render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });

    let sync!: ReturnType<typeof automaticSync>;
    act(() => {
      sync = automaticSync();
    });

    expect(within(syncRegion()).getByRole('button', { name: 'Syncing…' })).toBeDisabled();
    act(() => sync.finish());
    expect(await within(syncRegion()).findByText('Last synced Nov 25, 2026, 12:00 PM')).toBeInTheDocument();
    expect(within(syncRegion()).getByRole('button', { name: 'Sync now' })).toBeEnabled();
    await screen.findByText('Portal');
    expect(gameTitles()).toEqual(['Portal', 'Portal 2']);
  });

  it('says what went wrong when it fails, and changes nothing', async () => {
    const { operations, automaticSync } = fakeOperations({
      before: [steamGame(620, 'Portal 2', true, 1234, LAST_WEEK)],
      lastSync: LAST_WEEK,
      outcome: { outcome: 'steamUnreachable' },
    });
    render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });

    act(() => automaticSync().finish());

    expect(await within(syncRegion()).findByRole('alert')).toHaveTextContent(
      'Steam could not be reached. Nothing was changed. Try again later.',
    );
    expect(within(syncRegion()).getByText('Last synced Nov 20, 2026, 8:15 PM')).toBeInTheDocument();
    expect(gameTitles()).toEqual(['Portal 2']);
  });

  it('says that the Sync failed when the core reports an error', async () => {
    const { operations, automaticSync } = fakeOperations({ lastSync: LAST_WEEK });
    render(<App operations={operations} lang="en" />);
    await within(syncRegion()).findByText('Last synced Nov 20, 2026, 8:15 PM');

    act(() => automaticSync().fail());

    expect(await within(syncRegion()).findByRole('alert')).toHaveTextContent('The sync failed. Nothing was changed.');
    expect(within(syncRegion()).getByRole('button', { name: 'Sync now' })).toBeEnabled();
  });

  it('is no longer listened to once the interface is gone', async () => {
    const { operations, listeners } = fakeOperations();
    const { unmount } = render(<App operations={operations} lang="en" />);
    await within(syncRegion()).findByText('Not synced yet');
    expect(listeners.size).toBe(1);

    unmount();

    expect(listeners.size).toBe(0);
  });
});

/** The user enters a Steam Web API key in the settings and has it checked. */
async function storeKey() {
  const user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'Settings' }));
  const section = screen.getByRole('region', { name: 'Steam Web API key' });
  await user.type(await within(section).findByLabelText('Your key'), '0123456789ABCDEF0123456789ABCDEF');
  await user.click(within(section).getByRole('button', { name: 'Check and save' }));
  return section;
}

describe('Storing a Steam Web API key', () => {
  it('starts a Sync right away, which the sidebar shows like any automatic Sync', async () => {
    const { operations, calls } = fakeOperations({
      synced: [steamGame(400, 'Portal', false, 95, LAST_WEEK)],
      lastSync: null,
    });
    render(<App operations={operations} lang="en" />);
    await within(syncRegion()).findByText('Not synced yet');

    const section = await storeKey();

    expect(await within(section).findByRole('status')).toHaveTextContent('Steam confirmed your key. It is saved.');
    expect(await within(syncRegion()).findByRole('button', { name: 'Syncing…' })).toBeDisabled();
    act(() => calls.afterKeyStored!.finish());
    expect(await within(syncRegion()).findByText('Last synced Nov 25, 2026, 12:00 PM')).toBeInTheDocument();
    expect(within(syncRegion()).getByRole('button', { name: 'Sync now' })).toBeEnabled();
  });

  it('shows in the sidebar what went wrong in that Sync', async () => {
    const { operations, calls } = fakeOperations({ outcome: { outcome: 'gameDetailsPrivate' } });
    render(<App operations={operations} lang="en" />);
    await within(syncRegion()).findByText('Not synced yet');

    await storeKey();
    await within(syncRegion()).findByRole('button', { name: 'Syncing…' });
    act(() => calls.afterKeyStored!.finish());

    expect(await within(syncRegion()).findByRole('alert')).toBeInTheDocument();
    expect(within(syncRegion()).getByText('Not synced yet')).toBeInTheDocument();
  });

  it.each([{ outcome: 'invalid' }, { outcome: 'steamUnreachable' }] as const)(
    'starts no Sync when the key is $outcome',
    async (keyCheck) => {
      const { operations, calls } = fakeOperations({ keyCheck });
      render(<App operations={operations} lang="en" />);
      await within(syncRegion()).findByText('Not synced yet');

      const section = await storeKey();

      expect(await within(section).findByRole('alert')).toBeInTheDocument();
      expect(calls.afterKeyStored).toBeUndefined();
      expect(calls.syncs).toBe(0);
      expect(within(syncRegion()).getByRole('button', { name: 'Sync now' })).toBeEnabled();
    },
  );
});

describe('Playtime and Last Played in the Library', () => {
  it("shows Steam's Playtime and Last Played under each Game", async () => {
    render(
      <App
        operations={
          fakeOperations({
            before: [
              steamGame(70, 'Half-Life', false, 0, null),
              steamGame(400, 'Portal', false, 45, LAST_WEEK),
              steamGame(620, 'Portal 2', true, 1234, LAST_WEEK),
            ],
          }).operations
        }
        lang="en"
      />,
    );

    const [halfLife, portal, portal2] = within(await screen.findByRole('list', { name: 'Games' })).getAllByRole(
      'listitem',
    );
    expect(within(halfLife).getByText('Never played')).toBeInTheDocument();
    expect(within(portal).getByText('45 min played · last Nov 20, 2026')).toBeInTheDocument();
    expect(within(portal2).getByText('20.6 h played · last Nov 20, 2026')).toBeInTheDocument();
  });

  it('shows them in German', async () => {
    render(
      <App operations={fakeOperations({ before: [steamGame(620, 'Portal 2', true, 1234, LAST_WEEK)] }).operations} lang="de" />,
    );

    expect(await screen.findByText('20,6 Std. gespielt · zuletzt 20.11.2026')).toBeInTheDocument();
  });

  it('shows nothing about Playtime while it is not known', async () => {
    render(<App operations={fakeOperations({ before: [steamGame(620, 'Portal 2', true, null, null)] }).operations} lang="en" />);

    const [portal2] = within(await screen.findByRole('list', { name: 'Games' })).getAllByRole('listitem');
    // " played" is in every Playtime text, but not in the Status "Unplayed".
    expect(within(portal2).queryByText(/ played/)).not.toBeInTheDocument();
  });
});
