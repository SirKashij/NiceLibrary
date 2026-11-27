import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { DataImport, Game, Language, Operations } from './operations';

const CELESTE: Game = {
  id: 'local:4',
  title: 'Celeste',
  source: 'local',
  installed: true,
  playtimeMinutes: 90,
  lastPlayed: null,
  status: 'playing',
  favorite: false,
  progress: null,
};

/**
 * Fake operations of an empty Library whose import, once it ran, brings
 * back Celeste and the German interface; imports are counted.
 */
function fakeOperations(answer: () => Promise<DataImport> = async () => IMPORTED) {
  const calls = { imported: 0 };
  let games: Game[] = [];
  let language: Language = 'system';
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
    exportData: async () => ({ outcome: 'cancelled' }),
  };
  operations.library = async () => games;
  operations.filterLibrary = async () => games;
  operations.readInstalledGames = async () => games;
  operations.language = async () => language;
  operations.importData = () => {
    calls.imported += 1;
    return answer().then((outcome) => {
      if (outcome.outcome === 'imported') {
        games = [CELESTE];
        language = 'de';
      }
      return outcome;
    });
  };
  return { operations, calls };
}

const IMPORTED: DataImport = { outcome: 'imported', file: 'D:\\Backups\\NiceLibrary-2027-01-15.json', localGames: 1 };

const pending = <T,>() => new Promise<T>(() => {});

async function openBackup(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: lang === 'en' ? 'Backup' : 'Datensicherung' }) };
}

describe('Importing the data', () => {
  it('says what the import replaces and what stays, and asks before it imports anything', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openBackup(operations);

    await user.click(within(section).getByRole('button', { name: 'Import data…' }));

    const question = within(section).getByRole('alertdialog', { name: 'Replace your data with an Export File?' });
    expect(question).toHaveTextContent(
      'The import replaces your Local Games with their launch options, cover images and progress, your sessions, playtime adjustments, statuses, favorites, hidden games, Collections and settings with those in the file.',
    );
    expect(question).toHaveTextContent('What is here now is lost. Export it first if you want to keep it.');
    expect(question).toHaveTextContent(
      'Your Steam games stay, as they come from Steam, and your Steam Web API key is not touched.',
    );
    expect(calls.imported).toBe(0);
  });

  it('imports nothing when the user keeps their data', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openBackup(operations);
    await user.click(within(section).getByRole('button', { name: 'Import data…' }));

    await user.click(within(section).getByRole('button', { name: 'Keep my data' }));

    expect(within(section).queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(calls.imported).toBe(0);
  });

  it('after the import shows the imported data and settings, and says where they came from', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openBackup(operations);
    await user.click(within(section).getByRole('button', { name: 'Import data…' }));

    await user.click(within(section).getByRole('button', { name: 'Choose a file and replace' }));

    expect(calls.imported).toBe(1);
    // The imported language shows at once, and the notice with it.
    const backup = await screen.findByRole('region', { name: 'Datensicherung' });
    expect(await within(backup).findByRole('status')).toHaveTextContent(
      'Deine Daten wurden aus D:\\Backups\\NiceLibrary-2027-01-15.json importiert. Lokale Spiele: 1.',
    );
    await user.click(screen.getByRole('button', { name: 'Bibliothek' }));
    expect(await screen.findByRole('button', { name: 'Celeste spielen' })).toBeInTheDocument();
  });

  it('shows that the import runs and is not started twice meanwhile', async () => {
    const { operations, calls } = fakeOperations(pending);
    const { user, section } = await openBackup(operations);
    await user.click(within(section).getByRole('button', { name: 'Import data…' }));

    await user.click(within(section).getByRole('button', { name: 'Choose a file and replace' }));

    expect(within(section).getByRole('status')).toHaveTextContent('Importing…');
    expect(within(section).getByRole('button', { name: 'Import data…' })).toBeDisabled();
    expect(within(section).queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(calls.imported).toBe(1);
  });

  it('says nothing when the user cancels the file dialog', async () => {
    const { operations } = fakeOperations(async () => ({ outcome: 'cancelled' }));
    const { user, section } = await openBackup(operations);
    await user.click(within(section).getByRole('button', { name: 'Import data…' }));

    await user.click(within(section).getByRole('button', { name: 'Choose a file and replace' }));

    await waitFor(() => expect(within(section).getByRole('button', { name: 'Import data…' })).toBeEnabled());
    expect(within(section).queryByRole('status')).not.toBeInTheDocument();
    expect(within(section).queryByRole('alert')).not.toBeInTheDocument();
  });

  it.each<[string, DataImport | Error, string]>([
    [
      'a file of a newer version',
      { outcome: 'newerVersion', version: 2 },
      'This file was made by a newer version of NiceLibrary. Update the app to import it. Nothing was changed.',
    ],
    [
      'a damaged file',
      { outcome: 'damaged' },
      'This file is not an Export File of NiceLibrary, or it is damaged. Nothing was changed.',
    ],
    ['a failed import', new Error('cannot read D:\\backup.json'), 'Your data could not be imported.'],
  ])('says so for %s', async (_, answer, text) => {
    const { operations } = fakeOperations(() => (answer instanceof Error ? Promise.reject(answer) : Promise.resolve(answer)));
    const { user, section } = await openBackup(operations);
    await user.click(within(section).getByRole('button', { name: 'Import data…' }));

    await user.click(within(section).getByRole('button', { name: 'Choose a file and replace' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent(text);
    expect(screen.getByRole('region', { name: 'Backup' })).toBe(section);
  });

  it('speaks German', async () => {
    const { operations } = fakeOperations(async () => ({ outcome: 'damaged' }));
    const { user, section } = await openBackup(operations, 'de');

    await user.click(within(section).getByRole('button', { name: 'Daten importieren …' }));
    const question = within(section).getByRole('alertdialog', { name: 'Deine Daten durch eine Exportdatei ersetzen?' });
    expect(question).toHaveTextContent('Was jetzt hier ist, geht verloren.');
    await user.click(within(section).getByRole('button', { name: 'Datei wählen und ersetzen' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent(
      'Diese Datei ist keine Exportdatei von NiceLibrary, oder sie ist beschädigt. Es wurde nichts geändert.',
    );
  });
});
