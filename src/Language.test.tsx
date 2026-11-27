import { cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, Language, Operations, TrayMenuTexts } from './operations';

/** A promise that never settles: an operation still running. */
const pending = <T,>() => new Promise<T>(() => {});

/** A Steam Game played for an hour and a half, last on 5 March 2026. */
const played: Game = {
  id: 'steam:620',
  title: 'Portal 2',
  source: 'steam',
  installed: true,
  playtimeMinutes: 90,
  lastPlayed: Date.UTC(2026, 2, 5, 12) / 1000,
  status: 'playing',
  progress: null,
  favorite: false,
};

/** Fake operations that keep the interface language like the core does: the system's until the user chooses. */
function fakeOperations({
  stored = 'system' as Language,
  language = undefined as (() => Promise<Language>) | undefined,
  chooseLanguage = undefined as ((language: Language) => Promise<void>) | undefined,
} = {}) {
  const calls = { chosen: [] as Language[], trayMenus: [] as TrayMenuTexts[] };
  let kept = stored;
  const operations: Operations = {
    library: async () => [played],
    readInstalledGames: async () => [played],
    steamInstallation: async () => ({ state: 'notFound' }),
    exportData: async () => ({ outcome: 'cancelled' }),
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
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
    onAutomaticSync: () => () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async (texts) => {
      calls.trayMenus.push(texts);
    },
    filterLibrary: async () => [played],
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    game: async () => null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    onSession: () => () => {},
    openLogFolder: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
    importData: async () => ({ outcome: 'cancelled' }),
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    install: async () => ({ outcome: 'started' }),
    coverHue: async () => null,
    language: language ?? (async () => kept),
    chooseLanguage:
      chooseLanguage ??
      (async (chosen) => {
        calls.chosen.push(chosen);
        kept = chosen;
      }),
    limitedMode: async () => ({ outcome: 'off' }),
    openSteamPrivacySettings: async () => {},
    sessions: async () => [],
  };
  return { operations, calls };
}

const settingsButton = { en: 'Settings', de: 'Einstellungen' };
const languageSection = { en: 'Language', de: 'Sprache' };

async function openLanguageSettings(operations: Operations, system: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={system} />);
  await user.click(screen.getByRole('button', { name: settingsButton[system] }));
  return { user, section: () => screen.getByRole('region', { name: /^(Language|Sprache)$/ }) };
}

afterEach(() => {
  document.documentElement.removeAttribute('lang');
});

describe('The interface language', () => {
  it('follows the system language on first start', async () => {
    const { operations } = fakeOperations();
    const { section } = await openLanguageSettings(operations, 'de');

    expect(within(section()).getByRole('radio', { name: 'Wie im System' })).toBeChecked();
    expect(screen.getByRole('region', { name: languageSection.de })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe('de');
  });

  it('offers each language by its own name, in either interface language', async () => {
    for (const system of ['en', 'de'] as const) {
      const { operations } = fakeOperations();
      const { section } = await openLanguageSettings(operations, system);

      expect(within(section()).getByRole('radio', { name: 'Deutsch' })).toBeInTheDocument();
      expect(within(section()).getByRole('radio', { name: 'English' })).toBeInTheDocument();
      cleanup();
    }
  });

  it('switches at once, without a restart, and keeps the choice', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openLanguageSettings(operations, 'en');

    await user.click(within(section()).getByRole('radio', { name: 'Deutsch' }));

    expect(screen.getByRole('button', { name: 'Einstellungen' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Sprache' })).toBeInTheDocument();
    expect(within(section()).getByRole('radio', { name: 'Deutsch' })).toBeChecked();
    expect(document.documentElement.lang).toBe('de');
    expect(calls.chosen).toEqual(['de']);

    await user.click(within(section()).getByRole('radio', { name: 'English' }));

    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe('en');
    expect(calls.chosen).toEqual(['de', 'en']);
  });

  it('starts in the language chosen before, whatever the system language is', async () => {
    const { operations } = fakeOperations({ stored: 'de' });
    render(<App operations={operations} lang="en" />);

    expect(await screen.findByRole('button', { name: 'Einstellungen' })).toBeInTheDocument();
  });

  it('follows the system language again once the user chooses that', async () => {
    const { operations, calls } = fakeOperations({ stored: 'en' });
    const { user, section } = await openLanguageSettings(operations, 'de');
    await within(section()).findByRole('radio', { name: 'English', checked: true });

    await user.click(within(section()).getByRole('radio', { name: 'Match system' }));

    expect(screen.getByRole('button', { name: 'Einstellungen' })).toBeInTheDocument();
    expect(calls.chosen).toEqual(['system']);
  });

  it('writes Playtimes, numbers and dates the way the chosen language writes them', async () => {
    const { operations } = fakeOperations();
    const user = userEvent.setup();
    render(<App operations={operations} lang="en" />);
    expect(await screen.findByText('1.5 h played · last Mar 5, 2026')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Settings' }));
    await user.click(screen.getByRole('radio', { name: 'Deutsch' }));
    await user.click(screen.getByRole('button', { name: 'Bibliothek' }));

    expect(await screen.findByText('1,5 Std. gespielt · zuletzt 05.03.2026')).toBeInTheDocument();
  });

  it('gives the Tray its menu in the newly chosen language', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openLanguageSettings(operations, 'en');
    await waitFor(() => expect(calls.trayMenus).toHaveLength(1));

    await user.click(within(section()).getByRole('radio', { name: 'Deutsch' }));

    await waitFor(() =>
      expect(calls.trayMenus.at(-1)).toEqual({ tooltip: 'NiceLibrary', open: 'NiceLibrary öffnen', quit: 'Beenden' }),
    );
  });

  it('switches the texts of every part of the window, not only the settings', async () => {
    const { operations } = fakeOperations();
    const { user, section } = await openLanguageSettings(operations, 'en');

    await user.click(within(section()).getByRole('radio', { name: 'Deutsch' }));
    await user.click(screen.getByRole('button', { name: 'Bibliothek' }));

    expect(screen.getByRole('button', { name: 'Über NiceLibrary' })).toBeInTheDocument();
    expect(await screen.findByRole('button', { name: 'Favoriten, 0 Spiele' })).toBeInTheDocument();
    expect(await screen.findByRole('group', { name: 'Ansicht' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Portal 2 als Favorit' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'About NiceLibrary' })).not.toBeInTheDocument();
  });

  it('stays in the system language while the stored choice cannot be read', async () => {
    const { operations } = fakeOperations({ language: () => Promise.reject(new Error('store locked')) });
    const { section } = await openLanguageSettings(operations, 'de');

    expect(within(section()).getByRole('radio', { name: 'Wie im System' })).toBeChecked();
  });

  it('keeps a choice made before the stored one answers', async () => {
    const { operations } = fakeOperations({ language: pending });
    const { user, section } = await openLanguageSettings(operations, 'en');

    await user.click(within(section()).getByRole('radio', { name: 'Deutsch' }));

    expect(within(section()).getByRole('radio', { name: 'Deutsch' })).toBeChecked();
  });

  it('goes back and says so when the choice cannot be kept', async () => {
    const { operations } = fakeOperations({ chooseLanguage: () => Promise.reject(new Error('disk full')) });
    const { user, section } = await openLanguageSettings(operations, 'en');

    await user.click(within(section()).getByRole('radio', { name: 'Deutsch' }));

    expect(await within(section()).findByRole('alert')).toHaveTextContent('The language could not be saved.');
    expect(within(section()).getByRole('radio', { name: 'Match system' })).toBeChecked();
    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
  });
});
