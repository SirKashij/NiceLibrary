import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { AutomaticSteamAppListFetch, Operations, SteamAppList, SteamAppListFetch } from './operations';

/** 2026-11-25 12:00 UTC, in seconds since the Unix epoch. */
const NOW = 1_795_608_000;
/** 2026-11-20 20:15 UTC. */
const LAST_WEEK = 1_795_205_700;

const NEVER_FETCHED: SteamAppList = { titles: 0, fetchedAt: null, unfinished: false };

/** A promise and the function that settles it: an operation that runs until the test ends it. */
function running<T>() {
  let settle!: (value: T) => void;
  let fail!: () => void;
  const promise = new Promise<T>((resolve, reject) => {
    settle = resolve;
    fail = () => reject(new Error('database: disk full'));
  });
  return { promise, settle, fail };
}

/**
 * Fake operations that keep the Steam App List like the core: a fetch
 * answers `outcome` and, once it fetched to the end, the list is `after`.
 * `automatic` runs a fetch the core started on its own.
 */
function fakeOperations({
  before = NEVER_FETCHED,
  after = { titles: 250_000, fetchedAt: NOW, unfinished: false } as SteamAppList,
  outcome = { outcome: 'fetched', at: NOW } as SteamAppListFetch | 'failed' | 'running',
} = {}) {
  const calls = { fetches: 0 };
  let list = before;
  let fetch = running<SteamAppListFetch>();
  const listeners = new Set<(event: AutomaticSteamAppListFetch) => void>();
  const heard = (event: AutomaticSteamAppListFetch) => act(() => listeners.forEach((listener) => listener(event)));
  const operations: Operations = {
    library: async () => [],
    readInstalledGames: async () => [],
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => true,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    onAutomaticSync: () => () => {},
    filterLibrary: async () => [],
    game: async () => null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    install: async () => ({ outcome: 'started' }),
    openLogFolder: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    steamAppList: async () => list,
    fetchSteamAppList: () => {
      calls.fetches += 1;
      fetch = running<SteamAppListFetch>();
      if (outcome === 'failed') fetch.fail();
      else if (outcome !== 'running') {
        if (outcome.outcome === 'fetched') list = after;
        fetch.settle(outcome);
      }
      return fetch.promise;
    },
    onAutomaticSteamAppListFetch: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  const automatic = {
    start: () => heard({ state: 'started' }),
    finish: (answer: SteamAppListFetch) => {
      if (answer.outcome === 'fetched') list = after;
      return heard({ state: 'finished', ...answer });
    },
    fail: () => heard({ state: 'failed', reason: 'database: disk full' }),
  };
  return { operations, calls, automatic };
}

async function openAppListSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: lang === 'en' ? 'Steam app list' : 'Steam-App-Liste' }) };
}

describe('The Steam App List in the settings', () => {
  it('says why the list is fetched and that it was not fetched yet', async () => {
    const { section } = await openAppListSettings(fakeOperations().operations);

    expect(within(section).getByText(/to find their covers/)).toBeInTheDocument();
    expect(await within(section).findByText('Not fetched yet.')).toBeInTheDocument();
  });

  it('shows how many titles are stored and when the list was fetched', async () => {
    const { operations } = fakeOperations({ before: { titles: 213_402, fetchedAt: LAST_WEEK, unfinished: false } });
    const { section } = await openAppListSettings(operations);

    expect(await within(section).findByText('213,402 titles, fetched Nov 20, 2026, 8:15 PM')).toBeInTheDocument();
  });

  it('says that an unfinished fetch continues where it stopped', async () => {
    const { operations } = fakeOperations({ before: { titles: 30_000, fetchedAt: null, unfinished: true } });
    const { section } = await openAppListSettings(operations);

    expect(
      await within(section).findByText('30,000 titles so far. The next fetch continues where the last one stopped.'),
    ).toBeInTheDocument();
  });

  it('fetches the list on demand and shows the new state', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openAppListSettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Fetch now' }));

    expect(calls.fetches).toBe(1);
    expect(await within(section).findByText('250,000 titles, fetched Nov 25, 2026, 12:00 PM')).toBeInTheDocument();
    expect(within(section).getByRole('status')).toHaveTextContent('Steam’s app list is up to date.');
  });

  it('shows that a fetch runs and asks for no second one meanwhile', async () => {
    const { operations, calls } = fakeOperations({ outcome: 'running' });
    const { user, section } = await openAppListSettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Fetch now' }));

    const button = within(section).getByRole('button', { name: 'Fetching…' });
    expect(button).toBeDisabled();
    expect(within(section).getByRole('status')).toHaveTextContent('Fetching the app list from Steam…');
    await user.click(button);
    expect(calls.fetches).toBe(1);
  });

  it.each([
    [{ outcome: 'noKey' }, 'Add your Steam Web API key above to fetch the app list.'],
    [{ outcome: 'keyRejected' }, 'Steam rejected your Steam Web API key. Replace it above.'],
    [
      { outcome: 'steamUnreachable' },
      'Steam could not be reached. The titles fetched so far are kept, and the next fetch continues where this one stopped.',
    ],
    ['failed', 'The app list could not be fetched.'],
  ] as const)('says why a fetch did not bring the list: %j', async (outcome, message) => {
    const { operations } = fakeOperations({ outcome });
    const { user, section } = await openAppListSettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Fetch now' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent(message);
    expect(within(section).getByRole('button', { name: 'Fetch now' })).toBeEnabled();
  });

  it('follows a fetch the app starts on its own', async () => {
    const { operations, automatic } = fakeOperations();
    const { section } = await openAppListSettings(operations);
    expect(await within(section).findByText('Not fetched yet.')).toBeInTheDocument();

    automatic.start();
    expect(within(section).getByRole('button', { name: 'Fetching…' })).toBeDisabled();

    automatic.finish({ outcome: 'fetched', at: NOW });
    expect(await within(section).findByText('250,000 titles, fetched Nov 25, 2026, 12:00 PM')).toBeInTheDocument();
    expect(within(section).getByRole('button', { name: 'Fetch now' })).toBeEnabled();
  });

  it('tells when a fetch the app started on its own failed', async () => {
    const { operations, automatic } = fakeOperations();
    const { section } = await openAppListSettings(operations);

    automatic.start();
    automatic.fail();

    expect(await within(section).findByRole('alert')).toHaveTextContent('The app list could not be fetched.');
  });

  it('says when the state of the list could not be read', async () => {
    const { operations } = fakeOperations();
    operations.steamAppList = () => Promise.reject(new Error('database: disk full'));
    const { section } = await openAppListSettings(operations);

    expect(await within(section).findByRole('alert')).toHaveTextContent('The app list could not be checked.');
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations({ before: { titles: 213_402, fetchedAt: LAST_WEEK, unfinished: false } });
    const { user, section } = await openAppListSettings(operations, 'de');

    expect(await within(section).findByText('213.402 Titel, geladen am 20.11.2026, 20:15')).toBeInTheDocument();
    await user.click(within(section).getByRole('button', { name: 'Jetzt laden' }));
    expect(await within(section).findByRole('status')).toHaveTextContent('Steams App-Liste ist auf dem neuesten Stand.');
  });
});
