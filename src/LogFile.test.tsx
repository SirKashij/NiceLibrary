import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Operations } from './operations';

/** Fake operations that count how often the log folder is opened. */
function fakeOperations() {
  const calls = { logFolderOpened: 0 };
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
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'invalid' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    filterLibrary: async () => [],
    onAutomaticSync: () => () => {},
    game: async () => null,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    theme: async () => 'system',
    chooseTheme: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    onSession: () => () => {},
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
    openLogFolder: async () => {
      calls.logFolderOpened += 1;
    },
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
  };
  return { operations, calls };
}

async function openLogSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: lang === 'en' ? 'Log file' : 'Protokolldatei' }) };
}

describe('The log file for bug reports', () => {
  it('explains that nothing is sent and opens the log folder for the user', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openLogSettings(operations);

    expect(section).toHaveTextContent('Nothing is sent');
    await user.click(within(section).getByRole('button', { name: 'Open log folder' }));

    expect(calls.logFolderOpened).toBe(1);
  });

  it('says so when the log folder cannot be opened', async () => {
    const { operations } = fakeOperations();
    operations.openLogFolder = () => Promise.reject(new Error('no Explorer'));
    const { user, section } = await openLogSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Open log folder' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent('The log folder could not be opened.');
  });

  it('speaks German', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openLogSettings(operations, 'de');

    expect(section).toHaveTextContent('Nichts wird gesendet');
    await user.click(within(section).getByRole('button', { name: 'Protokollordner öffnen' }));

    expect(calls.logFolderOpened).toBe(1);
  });
});
