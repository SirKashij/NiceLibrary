import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, onTestFinished } from 'vitest';
import { App } from './App';
import type { CoverImageChoice, Game, Operations } from './operations';

const CELESTE: Game = {
  id: 'local:1',
  title: 'Celeste',
  source: 'local',
  installed: true,
  playtimeMinutes: 0,
  lastPlayed: null,
  status: 'unplayed',
  favorite: false,
};
const PORTAL_2: Game = {
  id: 'steam:620',
  title: 'Portal 2',
  source: 'steam',
  installed: true,
  playtimeMinutes: null,
  lastPlayed: null,
  status: 'unplayed',
  favorite: false,
};

/**
 * Fake operations over a Library with a Local Game and a Steam Game.
 * `choose` is what comes of the user choosing an image; by default they
 * choose one. Whether an image is chosen is kept as the core would keep it.
 */
function fakeOperations({
  choose = async () => ({ outcome: 'chosen' }),
  chosenAlready = false,
  remove = async () => {},
}: {
  choose?: () => Promise<CoverImageChoice>;
  chosenAlready?: boolean;
  remove?: () => Promise<void>;
} = {}) {
  const games = [CELESTE, PORTAL_2];
  const chosen = new Set<string>(chosenAlready ? [CELESTE.id] : []);
  const calls = { choose: [] as string[], remove: [] as string[] };
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game, revision) => `cover://localhost/${game.id}${revision ? `?revision=${revision}` : ''}`,
    chooseCoverImage: async (gameId) => {
      calls.choose.push(gameId);
      const answer = await choose();
      if (answer.outcome === 'chosen') chosen.add(gameId);
      return answer;
    },
    coverImageChosen: async (gameId) => chosen.has(gameId),
    removeCoverImage: async (gameId) => {
      calls.remove.push(gameId);
      await remove();
      chosen.delete(gameId);
    },
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    onSession: () => () => {},
    launch: async () => ({ outcome: 'launched' }),
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    onAutomaticSync: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    filterLibrary: async () => games,
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
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
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    install: async () => ({ outcome: 'started' }),
    coverHue: async (gameId) => (chosen.has(gameId) ? { hue: 220, saturation: 0.8 } : null),
    language: async () => 'system',
    chooseLanguage: async () => {},
    limitedMode: async () => ({ outcome: 'off' }),
    openSteamPrivacySettings: async () => {},
  };
  return { operations, calls };
}

async function open(title: string, label = `Details for ${title}`) {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: label }));
  const page = await screen.findByRole('region', { name: title });
  return { user, page };
}

describe('The Cover of a Local Game', () => {
  it('can be an image the user chooses, which then shows on the detail page and in the Library', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');
    expect(within(page).getByRole('img', { name: 'Cover of Celeste' })).toHaveAttribute(
      'src',
      'cover://localhost/local:1',
    );

    await user.click(within(page).getByRole('button', { name: 'Choose an image…' }));

    expect(calls.choose).toEqual(['local:1']);
    expect(await within(page).findByRole('img', { name: 'Cover of Celeste' })).toHaveAttribute(
      'src',
      'cover://localhost/local:1?revision=1',
    );
    await user.click(within(page).getByRole('button', { name: 'Back to the library' }));
    const grid = await screen.findByRole('list', { name: 'Games' });
    expect(within(grid).getByRole('img', { name: 'Cover of Celeste' })).toHaveAttribute(
      'src',
      'cover://localhost/local:1?revision=1',
    );
  });

  it('can go back to the executable’s icon once an image is chosen', async () => {
    const { operations, calls } = fakeOperations({ chosenAlready: true });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(await within(page).findByRole('button', { name: 'Use the executable’s icon' }));

    expect(calls.remove).toEqual(['local:1']);
    expect(await within(page).findByRole('img', { name: 'Cover of Celeste' })).toHaveAttribute(
      'src',
      'cover://localhost/local:1?revision=1',
    );
    expect(within(page).queryByRole('button', { name: 'Use the executable’s icon' })).not.toBeInTheDocument();
  });

  it('gives the detail page its Tint from the chosen image, and takes it back with the icon', async () => {
    // The theme colours as the app has them, from which the Tint is made.
    const tokens = document.createElement('style');
    tokens.textContent = readFileSync(join(__dirname, 'styles', 'tokens.css'), 'utf8');
    document.head.append(tokens);
    onTestFinished(() => tokens.remove());
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');
    expect(page).toHaveAttribute('data-tint', 'neutral');

    await user.click(within(page).getByRole('button', { name: 'Choose an image…' }));

    await waitFor(() => expect(page).toHaveAttribute('data-tint', 'cover'));

    await user.click(await within(page).findByRole('button', { name: 'Use the executable’s icon' }));

    await waitFor(() => expect(page).toHaveAttribute('data-tint', 'neutral'));
  });

  it('offers the way back to the icon only when an image is chosen', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');
    expect(within(page).queryByRole('button', { name: 'Use the executable’s icon' })).not.toBeInTheDocument();

    await user.click(within(page).getByRole('button', { name: 'Choose an image…' }));

    expect(await within(page).findByRole('button', { name: 'Use the executable’s icon' })).toBeInTheDocument();
  });

  it('stays as it was when the user cancels', async () => {
    const { operations } = fakeOperations({ choose: async () => ({ outcome: 'cancelled' }) });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Choose an image…' }));

    expect(within(page).getByRole('img', { name: 'Cover of Celeste' })).toHaveAttribute(
      'src',
      'cover://localhost/local:1',
    );
    expect(within(page).queryByRole('alert')).not.toBeInTheDocument();
  });

  it('says so when the file is not an image a Cover may be', async () => {
    const { operations } = fakeOperations({ choose: async () => ({ outcome: 'notAnImage' }) });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Choose an image…' }));

    expect(await within(page).findByRole('alert')).toHaveTextContent(
      'That file is not a PNG or JPEG image. The cover stays as it was.',
    );
  });

  it('says so when the image could not be chosen or the icon not restored', async () => {
    const { operations } = fakeOperations({
      chosenAlready: true,
      choose: () => Promise.reject(new Error('file dialog: no dialog today')),
      remove: () => Promise.reject(new Error('cannot remove')),
    });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await open('Celeste');

    await user.click(within(page).getByRole('button', { name: 'Choose an image…' }));
    expect(await within(page).findByRole('alert')).toHaveTextContent('The cover could not be changed.');

    await user.click(within(page).getByRole('button', { name: 'Use the executable’s icon' }));
    expect(await within(page).findByRole('alert')).toHaveTextContent('The cover could not be changed.');
    expect(within(page).getByRole('button', { name: 'Use the executable’s icon' })).toBeInTheDocument();
  });

  it('is not offered for a Steam Game, whose Cover comes from Steam', async () => {
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    const { page } = await open('Portal 2');

    expect(within(page).queryByRole('button', { name: 'Choose an image…' })).not.toBeInTheDocument();
  });

  it('is offered in German for a German interface', async () => {
    const { operations } = fakeOperations({ chosenAlready: true });
    render(<App operations={operations} lang="de" />);
    const { page } = await open('Celeste', 'Details zu Celeste');

    expect(within(page).getByRole('button', { name: 'Bild auswählen …' })).toBeInTheDocument();
    expect(await within(page).findByRole('button', { name: 'Symbol der Programmdatei verwenden' })).toBeInTheDocument();
  });
});
