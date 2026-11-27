import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { AutomaticSync, Game, Operations, SteamAccountChoice, SteamInstallation, SyncOutcome } from './operations';

const PLAYER_ONE = '76561197960287930';
const PLAYER_TWO = '76561197960287931';

const TWO_ACCOUNTS: SteamInstallation = {
  state: 'found',
  path: 'C:\\Steam',
  accounts: [
    { steamId: PLAYER_TWO, displayName: 'Player Two', mostRecent: true },
    { steamId: PLAYER_ONE, displayName: 'Player One', mostRecent: false },
  ],
};

const ONE_ACCOUNT: SteamInstallation = {
  state: 'found',
  path: 'C:\\Steam',
  accounts: [{ steamId: PLAYER_ONE, displayName: 'Player One', mostRecent: true }],
};

/**
 * Fake operations that remember the chosen Steam Account like the core does:
 * until a choice, the most recently used account; only 17-digit IDs starting
 * with 7656119 count as a SteamID. Each account has its own last Sync
 * (`lastSyncs`) and its own Library (`libraries`).
 */
function fakeOperations(
  steam: SteamInstallation,
  choose?: Operations['chooseSteamAccount'],
  { lastSyncs = {}, libraries = {} }: { lastSyncs?: Record<string, number>; libraries?: Record<string, Game[]> } = {},
) {
  const chosen: string[] = [];
  let account: string | null =
    steam.state === 'found' ? (steam.accounts.find((a) => a.mostRecent) ?? steam.accounts[0])?.steamId ?? null : null;
  const operations: Operations = {
    library: async () => (account && libraries[account]) || [],
    readInstalledGames: async () => (account && libraries[account]) || [],
    steamInstallation: async () => steam,
    steamAccount: async () => account,
    chooseSteamAccount:
      choose ??
      (async (steamId): Promise<SteamAccountChoice> => {
        const trimmed = steamId.trim();
        if (!/^7656119\d{10}$/.test(trimmed)) return { outcome: 'invalidSteamId' };
        chosen.push(trimmed);
        account = trimmed;
        return { outcome: 'chosen' };
      }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => (account && lastSyncs[account]) || null,
    filterLibrary: async () => [],
    onAutomaticSync: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    game: async () => null,
    launch: async () => ({ outcome: 'launched' }),
  };
  return { operations, chosen };
}

async function openSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, steam: screen.getByRole('region', { name: 'Steam' }) };
}

describe('Choosing the Steam Account', () => {
  it('preselects the most recently used of several accounts, and the user chooses another', async () => {
    const { operations, chosen } = fakeOperations(TWO_ACCOUNTS);
    const { user, steam } = await openSettings(operations);

    const choice = await within(steam).findByRole('radiogroup', { name: 'Account for your library' });
    expect(await within(choice).findByRole('radio', { name: 'Player Two' })).toBeChecked();
    expect(within(choice).getByRole('radio', { name: 'Player One' })).not.toBeChecked();

    await user.click(within(choice).getByRole('radio', { name: 'Player One' }));

    expect(chosen).toEqual([PLAYER_ONE]);
    expect(within(choice).getByRole('radio', { name: 'Player One' })).toBeChecked();
    expect(within(choice).getByRole('radio', { name: 'Player Two' })).not.toBeChecked();
  });

  it('preselects the only account', async () => {
    const { operations } = fakeOperations(ONE_ACCOUNT);
    const { steam } = await openSettings(operations);

    expect(await within(steam).findByRole('radio', { name: 'Player One' })).toBeChecked();
  });

  it('shows the chosen account after the settings are opened again', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS);
    const { user, steam } = await openSettings(operations);
    await user.click(await within(steam).findByRole('radio', { name: 'Player One' }));

    await user.click(screen.getByRole('button', { name: 'Library' }));
    await user.click(screen.getByRole('button', { name: 'Settings' }));

    const again = screen.getByRole('region', { name: 'Steam' });
    expect(await within(again).findByRole('radio', { name: 'Player One' })).toBeChecked();
  });

  it('keeps the previous account and says so when the choice cannot be saved', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS, () => Promise.reject(new Error('database: disk full')));
    const { user, steam } = await openSettings(operations);

    await user.click(await within(steam).findByRole('radio', { name: 'Player One' }));

    expect(await within(steam).findByRole('alert')).toHaveTextContent('The Steam account could not be saved.');
    expect(within(steam).getByRole('radio', { name: 'Player Two' })).toBeChecked();
    expect(within(steam).getByRole('radio', { name: 'Player One' })).not.toBeChecked();
  });

  it('names a SteamID in use that is none of the accounts on this computer', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS);
    operations.steamAccount = async () => '76561197960265729';
    const { steam } = await openSettings(operations);

    expect(await within(steam).findByRole('status')).toHaveTextContent(
      'Your library uses the SteamID 76561197960265729.',
    );
    expect(within(steam).getByRole('radio', { name: 'Player One' })).not.toBeChecked();
    expect(within(steam).getByRole('radio', { name: 'Player Two' })).not.toBeChecked();
  });

  it('says so when the account in use cannot be loaded', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS);
    operations.steamAccount = () => Promise.reject(new Error('database: locked'));
    const { steam } = await openSettings(operations);

    expect(await within(steam).findByRole('alert')).toHaveTextContent('The Steam account in use could not be loaded.');
  });

  it('reaches the accounts with the keyboard', async () => {
    const { operations, chosen } = fakeOperations(TWO_ACCOUNTS);
    const { user, steam } = await openSettings(operations);
    const selected = await within(steam).findByRole('radio', { name: 'Player Two' });
    await within(steam).findByRole('radio', { name: 'Player Two', checked: true });

    selected.focus();
    await user.keyboard('{ArrowDown}');

    expect(within(steam).getByRole('radio', { name: 'Player One' })).toHaveFocus();
    expect(chosen).toEqual([PLAYER_ONE]);
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS);
    const { steam } = await openSettings(operations, 'de');

    expect(await within(steam).findByRole('radiogroup', { name: 'Konto für deine Bibliothek' })).toBeInTheDocument();
  });
});

/** 2026-11-20 20:15 UTC, in seconds since the Unix epoch. */
const LAST_WEEK = 1_795_205_700;

const portal2: Game = { id: 'steam:620', title: 'Portal 2', source: 'steam', installed: true, playtimeMinutes: null, lastPlayed: null };
const halfLife: Game = { id: 'steam:70', title: 'Half-Life', source: 'steam', installed: true, playtimeMinutes: null, lastPlayed: null };

function syncRegion() {
  return within(screen.getByRole('complementary')).getByRole('region', { name: 'Sync' });
}

describe('After choosing another Steam Account', () => {
  it("shows that account's last Sync in the sidebar", async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS, undefined, { lastSyncs: { [PLAYER_ONE]: LAST_WEEK } });
    const { user, steam } = await openSettings(operations);
    await within(syncRegion()).findByText('Not synced yet');

    await user.click(await within(steam).findByRole('radio', { name: 'Player One' }));

    expect(await within(syncRegion()).findByText('Last synced Nov 20, 2026, 8:15 PM')).toBeInTheDocument();
  });

  it('says in the sidebar when that account has not been synced yet', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS, undefined, { lastSyncs: { [PLAYER_TWO]: LAST_WEEK } });
    const { user, steam } = await openSettings(operations);
    await within(syncRegion()).findByText('Last synced Nov 20, 2026, 8:15 PM');

    await user.click(await within(steam).findByRole('radio', { name: 'Player One' }));

    expect(await within(syncRegion()).findByText('Not synced yet')).toBeInTheDocument();
  });

  it('shows no answer of a Sync made for the account before', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS);
    let answer!: (outcome: SyncOutcome) => void;
    operations.sync = () => new Promise((resolve) => (answer = resolve));
    const { user, steam } = await openSettings(operations);
    await user.click(within(syncRegion()).getByRole('button', { name: 'Sync now' }));

    await user.click(await within(steam).findByRole('radio', { name: 'Player One' }));
    await within(syncRegion()).findByRole('button', { name: 'Sync now' });
    await act(async () => answer({ outcome: 'synced', at: LAST_WEEK }));

    expect(within(syncRegion()).getByText('Not synced yet')).toBeInTheDocument();
  });

  it('shows no answer of an automatic Sync that started for the account before', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS);
    const listeners = new Set<(event: AutomaticSync) => void>();
    const hear = (event: AutomaticSync) => act(() => listeners.forEach((listener) => listener(event)));
    operations.onAutomaticSync = (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    };
    const { user, steam } = await openSettings(operations);
    hear({ state: 'started' });

    await user.click(await within(steam).findByRole('radio', { name: 'Player One' }));
    await within(syncRegion()).findByText('Not synced yet');
    hear({ state: 'finished', outcome: 'synced', at: LAST_WEEK });

    expect(within(syncRegion()).getByText('Not synced yet')).toBeInTheDocument();
  });

  it("shows that account's Games in the Library", async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS, undefined, {
      libraries: { [PLAYER_ONE]: [portal2], [PLAYER_TWO]: [halfLife] },
    });
    const { user, steam } = await openSettings(operations);
    await user.click(await within(steam).findByRole('radio', { name: 'Player One' }));

    await user.click(screen.getByRole('button', { name: 'Library' }));

    expect(await screen.findByRole('heading', { name: 'Portal 2' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Half-Life' })).not.toBeInTheDocument();
  });
});

describe('Entering a SteamID by hand', () => {
  it('is offered when Steam is not found, and uses a valid SteamID', async () => {
    const { operations, chosen } = fakeOperations({ state: 'notFound' });
    const { user, steam } = await openSettings(operations);

    await user.type(await within(steam).findByRole('textbox', { name: 'SteamID' }), PLAYER_ONE);
    await user.click(within(steam).getByRole('button', { name: 'Use SteamID' }));

    expect(chosen).toEqual([PLAYER_ONE]);
    expect(await within(steam).findByRole('status')).toHaveTextContent(`Your library uses the SteamID ${PLAYER_ONE}.`);
    expect(within(steam).queryByRole('alert')).not.toBeInTheDocument();
  });

  it('is offered when no account has logged in to Steam', async () => {
    const { operations } = fakeOperations({ state: 'found', path: 'C:\\Steam', accounts: [] });
    const { steam } = await openSettings(operations);

    expect(await within(steam).findByRole('textbox', { name: 'SteamID' })).toBeInTheDocument();
    expect(within(steam).queryByRole('radiogroup')).not.toBeInTheDocument();
  });

  it('is not offered when accounts have logged in to Steam', async () => {
    const { operations } = fakeOperations(TWO_ACCOUNTS);
    const { steam } = await openSettings(operations);

    await within(steam).findByRole('radiogroup');
    expect(within(steam).queryByRole('textbox')).not.toBeInTheDocument();
  });

  it('shows the SteamID as the core stored it', async () => {
    const { operations } = fakeOperations({ state: 'notFound' });
    const { user, steam } = await openSettings(operations);

    await user.type(await within(steam).findByRole('textbox', { name: 'SteamID' }), `  ${PLAYER_ONE} `);
    await user.keyboard('{Enter}');

    await waitFor(() =>
      expect(within(steam).getByRole('status')).toHaveTextContent(`Your library uses the SteamID ${PLAYER_ONE}.`),
    );
  });

  it('refuses what is not a SteamID and says what one looks like', async () => {
    const { operations, chosen } = fakeOperations({ state: 'notFound' });
    const { user, steam } = await openSettings(operations);

    await user.type(await within(steam).findByRole('textbox', { name: 'SteamID' }), 'Player One');
    await user.keyboard('{Enter}');

    expect(await within(steam).findByRole('alert')).toHaveTextContent(
      'This is not a SteamID. A SteamID has 17 digits and starts with 7656119.',
    );
    expect(chosen).toEqual([]);
    expect(within(steam).queryByRole('status')).not.toBeInTheDocument();
  });

  it('shows the SteamID already in use', async () => {
    const { operations } = fakeOperations({ state: 'notFound' });
    operations.steamAccount = async () => PLAYER_ONE;
    const { steam } = await openSettings(operations);

    expect(await within(steam).findByRole('status')).toHaveTextContent(`Your library uses the SteamID ${PLAYER_ONE}.`);
    expect(within(steam).getByRole('textbox', { name: 'SteamID' })).toHaveValue(PLAYER_ONE);
  });

  it('says so when the SteamID cannot be saved', async () => {
    const { operations } = fakeOperations({ state: 'notFound' }, () => Promise.reject(new Error('database: disk full')));
    const { user, steam } = await openSettings(operations);

    await user.type(await within(steam).findByRole('textbox', { name: 'SteamID' }), PLAYER_ONE);
    await user.keyboard('{Enter}');

    expect(await within(steam).findByRole('alert')).toHaveTextContent('The Steam account could not be saved.');
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations({ state: 'notFound' });
    const { user, steam } = await openSettings(operations, 'de');

    await user.type(await within(steam).findByRole('textbox', { name: 'SteamID' }), 'abc');
    await user.click(within(steam).getByRole('button', { name: 'SteamID verwenden' }));

    expect(await within(steam).findByRole('alert')).toHaveTextContent(
      'Das ist keine SteamID. Eine SteamID hat 17 Ziffern und beginnt mit 7656119.',
    );
  });
});
