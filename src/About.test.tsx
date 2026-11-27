import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { About, AboutPage, Operations } from './operations';

/**
 * Fake operations that answer `about` and record which pages the About
 * dialog asks to open; `openPage` answers each of those requests.
 */
function fakeOperations({
  about = async (): Promise<About> => ({ version: '0.1.0', license: 'MIT' }),
  openPage = async () => {},
}: { about?: () => Promise<About>; openPage?: () => Promise<void> } = {}) {
  const calls = { opened: [] as AboutPage[] };
  const operations: Operations = {
    library: async () => [],
    readInstalledGames: async () => [],
    steamInstallation: async () => ({ state: 'notFound' }),
    exportData: async () => ({ outcome: 'cancelled' }),
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
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
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    theme: async () => 'system',
    chooseTheme: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    filterLibrary: async () => [],
    game: async () => null,
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
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
    coverHue: async () => null,
    language: async () => 'system',
    chooseLanguage: async () => {},
    about,
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    openAboutPage: async (page) => {
      calls.opened.push(page);
      return openPage();
    },
    install: async () => ({ outcome: 'started' }),
  };
  return { operations, calls };
}

async function openAbout(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'About NiceLibrary' : 'Über NiceLibrary' }));
  return { user, dialog: screen.getByRole('dialog', { name: lang === 'en' ? 'About NiceLibrary' : 'Über NiceLibrary' }) };
}

describe('The About dialog', () => {
  it('shows the version and the license', async () => {
    const { operations } = fakeOperations({ about: async () => ({ version: '0.2.3', license: 'MIT' }) });
    const { dialog } = await openAbout(operations);

    expect(await within(dialog).findByText('Version 0.2.3')).toBeInTheDocument();
    expect(within(dialog).getByText('MIT License')).toBeInTheDocument();
  });

  it('tells which connections the app makes and what it stores', async () => {
    const { operations } = fakeOperations();
    const { dialog } = await openAbout(operations);

    const privacy = within(dialog).getByRole('region', { name: 'Privacy' });
    expect(privacy).toHaveTextContent('Steam Web API');
    expect(privacy).toHaveTextContent('Steam’s image servers');
    expect(privacy).toHaveTextContent('GitHub');
    expect(privacy).toHaveTextContent('only on this computer');
    expect(privacy).toHaveTextContent('Windows Credential Manager');
    expect(privacy).toHaveTextContent('Nothing else leaves your computer.');
  });

  it('carries the Valve trademark notice and says the app is not affiliated with Valve', async () => {
    const { operations } = fakeOperations();
    const { dialog } = await openAbout(operations);

    expect(dialog).toHaveTextContent(
      'Steam and the Steam logo are trademarks and/or registered trademarks of Valve Corporation in the U.S. and/or other countries.',
    );
    expect(dialog).toHaveTextContent('NiceLibrary is not affiliated with, endorsed or sponsored by Valve Corporation.');
  });

  it('shows no logo, not the Steam logo either', async () => {
    const { operations } = fakeOperations();
    const { dialog } = await openAbout(operations);

    expect(within(dialog).queryByRole('img')).not.toBeInTheDocument();
    expect(dialog.querySelector('img, svg')).toBeNull();
  });

  it('opens the project page and the license in the browser', async () => {
    const { operations, calls } = fakeOperations();
    const { user, dialog } = await openAbout(operations);

    await user.click(within(dialog).getByRole('button', { name: 'Project page' }));
    await user.click(within(dialog).getByRole('button', { name: 'License' }));

    expect(calls.opened).toEqual(['project', 'license']);
  });

  it('says so when a page cannot be opened', async () => {
    const { operations } = fakeOperations({ openPage: () => Promise.reject(new Error('the browser crashed')) });
    const { user, dialog } = await openAbout(operations);

    await user.click(within(dialog).getByRole('button', { name: 'License' }));

    expect(await within(dialog).findByRole('alert')).toHaveTextContent('The page could not be opened.');
  });

  it('says so when the version cannot be read, and still shows the notices', async () => {
    const { operations } = fakeOperations({ about: () => Promise.reject(new Error('broken')) });
    const { dialog } = await openAbout(operations);

    expect(await within(dialog).findByRole('alert')).toHaveTextContent('The version could not be read.');
    expect(dialog).toHaveTextContent('not affiliated');
  });

  it('closes with its button and with Escape, and focus returns to where it was opened', async () => {
    const { operations } = fakeOperations();
    const { user, dialog } = await openAbout(operations);

    await user.click(within(dialog).getByRole('button', { name: 'Close' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'About NiceLibrary' })).toHaveFocus();

    await user.click(screen.getByRole('button', { name: 'About NiceLibrary' }));
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('puts the keyboard focus inside the dialog when it opens', async () => {
    const { operations } = fakeOperations();
    const { dialog } = await openAbout(operations);

    await waitFor(() => expect(dialog).toContainElement(document.activeElement as HTMLElement));
  });

  it('is in German too', async () => {
    const { operations } = fakeOperations();
    const { dialog } = await openAbout(operations, 'de');

    expect(await within(dialog).findByText('Version 0.1.0')).toBeInTheDocument();
    expect(within(dialog).getByText('MIT-Lizenz')).toBeInTheDocument();
    expect(within(dialog).getByRole('region', { name: 'Datenschutz' })).toHaveTextContent(
      'Sonst verlässt nichts deinen Computer.',
    );
    expect(dialog).toHaveTextContent(
      'Steam und das Steam-Logo sind Marken und/oder eingetragene Marken der Valve Corporation in den USA und/oder anderen Ländern.',
    );
    expect(dialog).toHaveTextContent(
      'NiceLibrary steht in keiner Verbindung zur Valve Corporation und wird von ihr weder unterstützt noch gesponsert.',
    );
    expect(within(dialog).getByRole('button', { name: 'Projektseite' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Lizenz' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Schließen' })).toBeInTheDocument();
  });
});
