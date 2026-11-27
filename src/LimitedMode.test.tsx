import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { AutomaticSync, Game, LimitedMode, Operations, SyncOutcome } from './operations';

/** 2026-11-25 12:00 UTC, in seconds since the Unix epoch. */
const NOW = 1_795_608_000;

const PORTAL_2: Game = {
  id: 'steam:620',
  title: 'Portal 2',
  source: 'steam',
  installed: true,
  playtimeMinutes: 75,
  lastPlayed: null,
  status: 'unplayed',
  favorite: false,
};

/**
 * Fake operations that behave like the core: the app runs in Limited Mode
 * for `reason` until a Sync answers `synced`; a Sync answers `outcome`.
 */
function fakeOperations({
  reason = 'noKey' as LimitedMode['outcome'],
  outcome = { outcome: 'synced', at: NOW } as SyncOutcome,
  privacyOpens = true,
} = {}) {
  const calls = { syncs: 0, privacySettingsOpened: 0 };
  let mode: LimitedMode = { outcome: reason };
  const listeners = new Set<(event: AutomaticSync) => void>();
  const operations: Operations = {
    library: async () => [PORTAL_2],
    readInstalledGames: async () => [PORTAL_2],
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => reason !== 'noKey',
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => {
      calls.syncs += 1;
      if (outcome.outcome === 'synced') mode = { outcome: 'off' };
      return outcome;
    },
    lastSync: async () => null,
    onAutomaticSync: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    filterLibrary: async () => [PORTAL_2],
    game: async () => PORTAL_2,
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    limitedMode: async () => mode,
    openSteamPrivacySettings: async () => {
      calls.privacySettingsOpened += 1;
      if (!privacyOpens) throw new Error('no browser');
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
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
  };
  /** The core runs an automatic Sync that ends with `answer`. */
  const automaticSync = (answer: SyncOutcome) => {
    listeners.forEach((listener) => listener({ state: 'started' }));
    if (answer.outcome === 'synced') mode = { outcome: 'off' };
    listeners.forEach((listener) => listener({ state: 'finished', ...answer }));
  };
  return { operations, calls, automaticSync };
}

function hint(name = 'Limited mode') {
  return screen.findByRole('region', { name });
}

describe('The Limited Mode hint', () => {
  it('does not appear while the app does not run in Limited Mode', async () => {
    render(<App operations={fakeOperations({ reason: 'off' }).operations} lang="en" />);

    await screen.findByRole('list', { name: 'Games' });
    expect(screen.queryByRole('region', { name: 'Limited mode' })).not.toBeInTheDocument();
  });

  it('without a key, says so and opens the setup of the key in the settings', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations({ reason: 'noKey' }).operations} lang="en" />);

    const region = await hint();
    expect(region).toHaveTextContent('Without a Steam Web API key, the app shows only your installed Steam games');
    await user.click(within(region).getByRole('button', { name: 'Set up key' }));

    expect(screen.getByRole('heading', { level: 1, name: 'Settings' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Steam Web API key' })).toBeInTheDocument();
  });

  it('with a rejected key, says so and opens the key in the settings', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations({ reason: 'keyRejected' }).operations} lang="en" />);

    const region = await hint();
    expect(region).toHaveTextContent('Steam rejected your Steam Web API key');
    await user.click(within(region).getByRole('button', { name: 'Open settings' }));

    expect(screen.getByRole('region', { name: 'Steam Web API key' })).toBeInTheDocument();
  });

  it('with private game details, explains the setting and opens Steam’s privacy settings', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations({ reason: 'gameDetailsPrivate' });
    render(<App operations={operations} lang="en" />);

    const region = await hint();
    expect(region).toHaveTextContent(
      '"Game details" is a privacy setting of your Steam profile: it decides who may see which games you own and how long you played them. Set it to "Public"',
    );
    await user.click(within(region).getByRole('button', { name: 'Open privacy settings' }));

    expect(calls.privacySettingsOpened).toBe(1);
    expect(within(region).queryByRole('alert')).not.toBeInTheDocument();
  });

  it('says when Steam’s privacy settings could not be opened', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations({ reason: 'gameDetailsPrivate', privacyOpens: false }).operations} lang="en" />);

    const region = await hint();
    await user.click(within(region).getByRole('button', { name: 'Open privacy settings' }));

    expect(await within(region).findByRole('alert')).toHaveTextContent('Your Steam privacy settings could not be opened.');
  });

  it('when Steam cannot be reached, says so and tries again, and disappears once the Sync succeeds', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations({ reason: 'steamUnreachable' });
    render(<App operations={operations} lang="en" />);

    const region = await hint();
    expect(region).toHaveTextContent('Steam could not be reached, so the app shows only your installed Steam games');
    await user.click(within(region).getByRole('button', { name: 'Try again' }));

    expect(calls.syncs).toBe(1);
    await waitForHintToGo();
  });

  it('stays, with the same reason, when trying again fails the same way', async () => {
    const user = userEvent.setup();
    const { operations, calls } = fakeOperations({ reason: 'steamUnreachable', outcome: { outcome: 'steamUnreachable' } });
    render(<App operations={operations} lang="en" />);

    await user.click(within(await hint()).getByRole('button', { name: 'Try again' }));

    expect(calls.syncs).toBe(1);
    expect(await within(await hint()).findByRole('button', { name: 'Try again' })).toBeEnabled();
  });

  it('disappears once Sync now in the sidebar succeeds', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations({ reason: 'gameDetailsPrivate' }).operations} lang="en" />);
    await hint();

    await user.click(within(screen.getByRole('complementary')).getByRole('button', { name: 'Sync now' }));

    await waitForHintToGo();
  });

  it('disappears once an automatic Sync succeeds', async () => {
    const { operations, automaticSync } = fakeOperations({ reason: 'keyRejected' });
    render(<App operations={operations} lang="en" />);
    await hint();

    act(() => automaticSync({ outcome: 'synced', at: NOW }));

    await waitForHintToGo();
  });

  it('speaks German', async () => {
    render(<App operations={fakeOperations({ reason: 'gameDetailsPrivate' }).operations} lang="de" />);

    const region = await hint('Eingeschränkter Modus');
    expect(region).toHaveTextContent('„Spieldetails“ ist eine Privatsphäre-Einstellung deines Steam-Profils');
    expect(within(region).getByRole('button', { name: 'Privatsphäre-Einstellungen öffnen' })).toBeInTheDocument();
    expect(within(region).getByRole('button', { name: 'Erneut synchronisieren' })).toBeInTheDocument();
  });
});

async function waitForHintToGo() {
  await screen.findByRole('list', { name: 'Games' });
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Limited mode' })).not.toBeInTheDocument());
}
