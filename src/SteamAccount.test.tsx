import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Operations, SteamAccountChoice, SteamInstallation } from './operations';

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
 * with 7656119 count as a SteamID.
 */
function fakeOperations(steam: SteamInstallation, choose?: Operations['chooseSteamAccount']) {
  const chosen: string[] = [];
  let account: string | null =
    steam.state === 'found' ? (steam.accounts.find((a) => a.mostRecent) ?? steam.accounts[0])?.steamId ?? null : null;
  const operations: Operations = {
    library: async () => [],
    readInstalledGames: async () => [],
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
