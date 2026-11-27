import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Operations } from './operations';

/** Fake operations whose export writes to a backup folder; exports are counted. */
function fakeOperations() {
  const calls = { exported: 0 };
  const operations: Operations = {
    library: async () => [],
    readInstalledGames: async () => [],
    steamInstallation: async () => ({ state: 'notFound' }),
    coverHue: async () => null,
    install: async () => ({ outcome: 'started' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
    importData: async () => ({ outcome: 'cancelled' }),
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
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
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
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
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
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    language: async () => 'system',
    chooseLanguage: async () => {},
    exportData: async () => {
      calls.exported += 1;
      return { outcome: 'exported', file: 'C:\\Backups\\NiceLibrary-2027-01-15.json' };
    },
  };
  return { operations, calls };
}

const pending = <T,>() => new Promise<T>(() => {});

async function openExportSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: lang === 'en' ? 'Backup' : 'Datensicherung' }) };
}

describe('Exporting the data', () => {
  it('says what the export holds and that the Steam Web API key is not in it', async () => {
    const { operations } = fakeOperations();
    const { section } = await openExportSettings(operations);

    expect(section).toHaveTextContent('Local Games');
    expect(section).toHaveTextContent('Collections');
    expect(section).toHaveTextContent('Your Steam Web API key is never exported');
  });

  it('exports when asked and says where the file is', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openExportSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Export data…' }));

    expect(calls.exported).toBe(1);
    expect(await within(section).findByRole('status')).toHaveTextContent(
      'Your data was exported to C:\\Backups\\NiceLibrary-2027-01-15.json.',
    );
  });

  it('shows that the export runs and is not started twice meanwhile', async () => {
    const { operations, calls } = fakeOperations();
    operations.exportData = () => {
      calls.exported += 1;
      return pending();
    };
    const { user, section } = await openExportSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Export data…' }));

    expect(within(section).getByRole('status')).toHaveTextContent('Exporting…');
    expect(within(section).getByRole('button', { name: 'Export data…' })).toBeDisabled();
    expect(calls.exported).toBe(1);
  });

  it('says nothing when the user cancels', async () => {
    const { operations } = fakeOperations();
    operations.exportData = async () => ({ outcome: 'cancelled' });
    const { user, section } = await openExportSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Export data…' }));

    await waitFor(() => expect(within(section).getByRole('button', { name: 'Export data…' })).toBeEnabled());
    expect(within(section).queryByRole('status')).not.toBeInTheDocument();
    expect(within(section).queryByRole('alert')).not.toBeInTheDocument();
  });

  it('says so when the export fails', async () => {
    const { operations } = fakeOperations();
    operations.exportData = () => Promise.reject(new Error('cannot write D:\\backup.json: disk full'));
    const { user, section } = await openExportSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Export data…' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent('Your data could not be exported.');
    expect(within(section).queryByRole('status')).not.toBeInTheDocument();
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations();
    const { user, section } = await openExportSettings(operations, 'de');

    expect(section).toHaveTextContent('Dein Steam-Web-API-Schlüssel wird nie exportiert');
    await user.click(within(section).getByRole('button', { name: 'Daten exportieren …' }));

    expect(await within(section).findByRole('status')).toHaveTextContent(
      'Deine Daten wurden nach C:\\Backups\\NiceLibrary-2027-01-15.json exportiert.',
    );
  });
});
