import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Operations, SteamWebApiKeyCheck } from './operations';

const VALID_KEY = '0123456789ABCDEF0123456789ABCDEF';
const UNKNOWN_KEY = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';

/** A promise that never settles: an operation still running. */
const pending = <T,>() => new Promise<T>(() => {});

/**
 * Fake operations that keep the key like the core does: Steam accepts only
 * VALID_KEY, and only an accepted key is stored.
 */
function fakeOperations({ stored = false } = {}) {
  const calls = { checked: [] as string[], removed: 0, pageOpened: 0 };
  let key: string | null = stored ? VALID_KEY : null;
  const operations: Operations = {
    library: async () => [],
    readInstalledGames: async () => [],
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => key !== null,
    storeSteamWebApiKey: async (pasted): Promise<SteamWebApiKeyCheck> => {
      calls.checked.push(pasted);
      if (pasted.trim() !== VALID_KEY) return { outcome: 'invalid' };
      key = VALID_KEY;
      return { outcome: 'stored' };
    },
    removeSteamWebApiKey: async () => {
      calls.removed += 1;
      key = null;
    },
    openSteamWebApiKeyPage: async () => {
      calls.pageOpened += 1;
    },
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    theme: async () => 'system',
    language: async () => 'system',
    chooseLanguage: async () => {},
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
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    onAutomaticSync: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    onSession: () => () => {},
    game: async () => null,
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    coverHue: async () => null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
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
  return { operations, calls };
}

async function openKeySettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: lang === 'en' ? 'Steam Web API key' : 'Steam-Web-API-Schlüssel' }) };
}

async function enterKey(user: ReturnType<typeof userEvent.setup>, section: HTMLElement, key: string) {
  await user.type(await within(section).findByLabelText('Your key'), key);
  await user.click(within(section).getByRole('button', { name: 'Check and save' }));
}

describe('Entering the Steam Web API key', () => {
  it('guides through creating a key on the Steam page', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openKeySettings(operations);

    expect(await within(section).findByText('No key is saved.')).toBeInTheDocument();
    const steps = within(section).getByRole('list', { name: 'How to get your key' });
    expect(within(steps).getAllByRole('listitem')).toHaveLength(3);

    await user.click(within(section).getByRole('button', { name: 'Open the Steam page' }));

    expect(calls.pageOpened).toBe(1);
    expect(section).toHaveTextContent('steamcommunity.com/dev/apikey');
  });

  it('says so when the Steam page cannot be opened', async () => {
    const { operations } = fakeOperations();
    operations.openSteamWebApiKeyPage = () => Promise.reject(new Error('no browser'));
    const { user, section } = await openKeySettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Open the Steam page' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent(
      'The Steam page could not be opened. Go to steamcommunity.com/dev/apikey in your browser.',
    );
  });

  it('confirms a valid key and shows that it is saved', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openKeySettings(operations);

    await enterKey(user, section, VALID_KEY);

    expect(calls.checked).toEqual([VALID_KEY]);
    expect(await within(section).findByRole('status')).toHaveTextContent('Steam confirmed your key. It is saved.');
    expect(within(section).getByText('A key is saved.')).toBeInTheDocument();
    expect(within(section).queryByLabelText('Your key')).not.toBeInTheDocument();
  });

  it('says a rejected key is invalid and keeps it out of the store', async () => {
    const { operations } = fakeOperations();
    const { user, section } = await openKeySettings(operations);

    await enterKey(user, section, UNKNOWN_KEY);

    expect(await within(section).findByRole('alert')).toHaveTextContent('This key is invalid.');
    expect(within(section).getByText('No key is saved.')).toBeInTheDocument();
    expect(within(section).getByLabelText('Your key')).toHaveValue(UNKNOWN_KEY);
  });

  it('does not call the key invalid when Steam cannot be reached', async () => {
    const { operations } = fakeOperations();
    operations.storeSteamWebApiKey = async () => ({ outcome: 'steamUnreachable' });
    const { user, section } = await openKeySettings(operations);

    await enterKey(user, section, VALID_KEY);

    const alert = await within(section).findByRole('alert');
    expect(alert).toHaveTextContent('Steam could not be reached, so your key could not be checked. It was not saved.');
    expect(alert).not.toHaveTextContent('invalid');
  });

  it('says so when the key cannot be saved', async () => {
    const { operations } = fakeOperations();
    operations.storeSteamWebApiKey = () => Promise.reject(new Error('secret store: access denied'));
    const { user, section } = await openKeySettings(operations);

    await enterKey(user, section, VALID_KEY);

    expect(await within(section).findByRole('alert')).toHaveTextContent('Your key could not be saved.');
  });

  it('shows that the key is being checked and takes no second key meanwhile', async () => {
    const { operations } = fakeOperations();
    operations.storeSteamWebApiKey = pending;
    const { user, section } = await openKeySettings(operations);

    await enterKey(user, section, VALID_KEY);

    expect(await within(section).findByRole('status')).toHaveTextContent('Checking your key with Steam…');
    expect(within(section).getByRole('button', { name: 'Check and save' })).toBeDisabled();
  });

  it('hides the key while it is typed and never shows it afterwards', async () => {
    const { operations } = fakeOperations();
    const { user, section } = await openKeySettings(operations);

    const field = await within(section).findByLabelText('Your key');
    expect(field).toHaveAttribute('type', 'password');
    expect(field).toHaveAttribute('autocomplete', 'off');

    await enterKey(user, section, VALID_KEY);
    await within(section).findByText('A key is saved.');

    expect(document.body).not.toHaveTextContent(VALID_KEY);
  });

  it('submits the key with Enter', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openKeySettings(operations);

    await user.type(await within(section).findByLabelText('Your key'), `${VALID_KEY}{Enter}`);

    expect(calls.checked).toEqual([VALID_KEY]);
  });

  it('says which Steam data the app stores', async () => {
    const { operations } = fakeOperations();
    const { section } = await openKeySettings(operations);

    const note = await within(section).findByRole('note', { name: 'What NiceLibrary stores' });
    for (const data of ['SteamID', 'games', 'playtime', 'achievements', 'covers', 'only on this computer']) {
      expect(note).toHaveTextContent(data);
    }
  });

  it('says what goes wrong when it cannot tell whether a key is saved', async () => {
    const { operations } = fakeOperations();
    operations.steamWebApiKeyStored = () => Promise.reject(new Error('secret store: locked'));
    const { section } = await openKeySettings(operations);

    expect(await within(section).findByRole('alert')).toHaveTextContent('Whether a key is saved could not be checked.');
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations();
    const { section } = await openKeySettings(operations, 'de');

    expect(await within(section).findByText('Es ist kein Schlüssel gespeichert.')).toBeInTheDocument();
    expect(within(section).getByRole('button', { name: 'Prüfen und speichern' })).toBeInTheDocument();
  });
});

describe('Replacing or removing the Steam Web API key', () => {
  it('replaces a saved key with a new one', async () => {
    const { operations, calls } = fakeOperations({ stored: true });
    const { user, section } = await openKeySettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Replace key' }));
    await enterKey(user, section, VALID_KEY);

    expect(calls.checked).toEqual([VALID_KEY]);
    expect(await within(section).findByRole('status')).toHaveTextContent('Steam confirmed your key. It is saved.');
  });

  it('keeps the saved key when replacing is cancelled', async () => {
    const { operations, calls } = fakeOperations({ stored: true });
    const { user, section } = await openKeySettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Replace key' }));
    await user.click(within(section).getByRole('button', { name: 'Cancel' }));

    expect(within(section).getByText('A key is saved.')).toBeInTheDocument();
    expect(calls.checked).toEqual([]);
  });

  it('keeps the saved key when its replacement is rejected', async () => {
    const { operations } = fakeOperations({ stored: true });
    const { user, section } = await openKeySettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Replace key' }));
    await enterKey(user, section, UNKNOWN_KEY);

    expect(await within(section).findByRole('alert')).toHaveTextContent('This key is invalid.');
    expect(within(section).getByText('A key is saved.')).toBeInTheDocument();
  });

  it('removes the saved key', async () => {
    const { operations, calls } = fakeOperations({ stored: true });
    const { user, section } = await openKeySettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Remove key' }));

    expect(calls.removed).toBe(1);
    expect(await within(section).findByRole('status')).toHaveTextContent('Your key was removed.');
    expect(within(section).getByText('No key is saved.')).toBeInTheDocument();
  });

  it('keeps showing the key as saved when it cannot be removed', async () => {
    const { operations } = fakeOperations({ stored: true });
    operations.removeSteamWebApiKey = () => Promise.reject(new Error('secret store: access denied'));
    const { user, section } = await openKeySettings(operations);

    await user.click(await within(section).findByRole('button', { name: 'Remove key' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent('Your key could not be removed.');
    expect(within(section).getByText('A key is saved.')).toBeInTheDocument();
  });
});
