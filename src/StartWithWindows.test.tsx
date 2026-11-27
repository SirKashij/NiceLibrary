import { fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Operations } from './operations';

/**
 * Fake operations whose start with Windows behaves like the autostart entry
 * the core reads: the test can remove it outside the app (`removeOutsideTheApp`).
 */
function fakeOperations({ startsWithWindows = false } = {}) {
  const calls = { startWithWindows: [] as boolean[] };
  let entry = startsWithWindows;
  const operations: Operations = {
    library: async () => [],
    readInstalledGames: async () => [],
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'invalid' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    onSession: () => () => {},
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    onAutomaticSync: () => () => {},
    filterLibrary: async () => [],
    game: async () => null,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    startsWithWindows: async () => entry,
    setStartWithWindows: async (on) => {
      calls.startWithWindows.push(on);
      entry = on;
    },
  };
  const removeOutsideTheApp = () => {
    entry = false;
  };
  return { operations, calls, removeOutsideTheApp };
}

async function openWindowSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: lang === 'en' ? 'Window' : 'Fenster' }) };
}

describe('Starting with Windows', () => {
  it('is off by default', async () => {
    const { operations } = fakeOperations();
    const { section } = await openWindowSettings(operations);

    expect(await within(section).findByRole('checkbox', { name: 'Start with Windows' })).not.toBeChecked();
  });

  it('says that the app then starts hidden in the tray', async () => {
    const { operations } = fakeOperations();
    const { section } = await openWindowSettings(operations);
    const checkbox = await within(section).findByRole('checkbox', { name: 'Start with Windows' });

    expect(checkbox).toHaveAccessibleDescription(
      'When you sign in to Windows, NiceLibrary starts in the tray without opening its window.',
    );
  });

  it('turns starting with Windows on and off again', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openWindowSettings(operations);
    const checkbox = await within(section).findByRole('checkbox', { name: 'Start with Windows' });

    await user.click(checkbox);
    expect(checkbox).toBeChecked();
    await user.click(checkbox);

    expect(checkbox).not.toBeChecked();
    expect(calls.startWithWindows).toEqual([true, false]);
  });

  it('shows it on when the autostart entry is there', async () => {
    const { operations } = fakeOperations({ startsWithWindows: true });
    const { section } = await openWindowSettings(operations);

    expect(await within(section).findByRole('checkbox', { name: 'Start with Windows' })).toBeChecked();
  });

  it('shows it off once the entry was removed outside the app and the window is back in front', async () => {
    const { operations, removeOutsideTheApp } = fakeOperations({ startsWithWindows: true });
    const { section } = await openWindowSettings(operations);
    const checkbox = await within(section).findByRole('checkbox', { name: 'Start with Windows' });
    expect(checkbox).toBeChecked();

    removeOutsideTheApp();
    await act(async () => {
      fireEvent.focus(window);
    });

    expect(checkbox).not.toBeChecked();
  });

  it('is named in German', async () => {
    const { operations } = fakeOperations();
    const { section } = await openWindowSettings(operations, 'de');

    const checkbox = await within(section).findByRole('checkbox', { name: 'Mit Windows starten' });
    expect(checkbox).toHaveAccessibleDescription(
      'Wenn du dich bei Windows anmeldest, startet NiceLibrary im Infobereich, ohne sein Fenster zu öffnen.',
    );
  });

  it('says so when it cannot be read', async () => {
    const { operations } = fakeOperations();
    operations.startsWithWindows = () => Promise.reject(new Error('autostart entry: access denied'));
    const { section } = await openWindowSettings(operations);

    expect(await within(section).findByRole('alert')).toHaveTextContent(
      'Whether NiceLibrary starts with Windows could not be read.',
    );
    expect(within(section).queryByRole('checkbox', { name: 'Start with Windows' })).not.toBeInTheDocument();
  });

  it('says so when it cannot be changed and keeps showing what Windows will do', async () => {
    const { operations } = fakeOperations();
    operations.setStartWithWindows = () => Promise.reject(new Error('autostart entry: access denied'));
    const { user, section } = await openWindowSettings(operations);
    const checkbox = await within(section).findByRole('checkbox', { name: 'Start with Windows' });

    await user.click(checkbox);

    expect(await within(section).findByRole('alert')).toHaveTextContent(
      'Starting with Windows could not be changed.',
    );
    expect(checkbox).not.toBeChecked();
  });
});
