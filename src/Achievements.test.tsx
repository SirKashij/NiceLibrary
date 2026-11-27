import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Achievement, Game, GameAchievements, Operations } from './operations';

function game(id: string, title: string, details: Partial<Game> = {}): Game {
  return { id, title, source: 'steam', installed: true, playtimeMinutes: 1_260, lastPlayed: null, progress: null, status: 'unplayed', favorite: false, ...details };
}

/** 2026-11-20 20:15:00 UTC, in seconds since the Unix epoch. */
const LAST_WEEK = 1_795_205_700;
/** 2026-10-01 18:00:00 UTC. */
const IN_OCTOBER = 1_790_877_600;

const PORTAL_2 = game('steam:620', 'Portal 2');

function achievement(id: string, name: string, unlockedAt: number | null, details: Partial<Achievement> = {}): Achievement {
  return {
    id,
    name,
    description: `How to get ${name}`,
    unlocked: unlockedAt !== null,
    unlockedAt,
    icon: `${id.toLowerCase()}${unlockedAt === null ? '-grey' : ''}`,
    ...details,
  };
}

const PORTAL_2_ACHIEVEMENTS: GameAchievements = {
  state: 'listed',
  unlocked: 2,
  total: 4,
  achievements: [
    achievement('ACH_FINISH', 'Lunacy', LAST_WEEK),
    achievement('ACH_WAKE_UP', 'Wake up call', IN_OCTOBER),
    achievement('ACH_SPEED_RUN', 'Speed run', null),
    achievement('ACH_PARTNER', 'Partner', null, { description: null, icon: null }),
  ],
};

/** Fake operations over a Library of `games`, whose Achievements are as `achievements` says. */
function fakeOperations(games: Game[], achievements: Record<string, GameAchievements>, sync?: () => void) {
  const asked: string[] = [];
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => {
      sync?.();
      return { outcome: 'synced', at: LAST_WEEK };
    },
    lastSync: async () => null,
    onAutomaticSync: () => () => {},
    filterLibrary: async () => games,
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    achievements: async (gameId) => {
      asked.push(gameId);
      const answer = achievements[gameId];
      return answer ?? Promise.reject(new Error('no such game'));
    },
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
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
    onSession: () => () => {},
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
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
  };
  return { operations, asked };
}

async function openDetailPage(title: string, label = `Details for ${title}`) {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: label }));
  return { user, page: await screen.findByRole('region', { name: title }) };
}

describe('The Achievements on the detail page of a Steam Game', () => {
  it('lists each Achievement with icon, name, description and unlock date, unlocked ones first', async () => {
    const { operations } = fakeOperations([PORTAL_2], { 'steam:620': PORTAL_2_ACHIEVEMENTS });
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Portal 2');

    const list = await within(page).findByRole('list', { name: 'Achievements of Portal 2' });
    const items = within(list).getAllByRole('listitem');
    expect(items.map((item) => within(item).getByRole('heading').textContent)).toEqual([
      'Lunacy',
      'Wake up call',
      'Speed run',
      'Partner',
    ]);
    expect(items[0]).toHaveTextContent('How to get Lunacy');
    expect(items[0]).toHaveTextContent('Unlocked Nov 20, 2026');
    expect(items[1]).toHaveTextContent('Unlocked Oct 1, 2026');
    expect(items[2]).toHaveTextContent('Locked');
    expect(items[0].querySelector('img')).toHaveAttribute('src', 'cover://localhost/steam:620/achievements/ach_finish');
    expect(items[2].querySelector('img')).toHaveAttribute('src', 'cover://localhost/steam:620/achievements/ach_speed_run-grey');
  });

  it('shows how many of all Achievements are unlocked', async () => {
    const { operations } = fakeOperations([PORTAL_2], { 'steam:620': PORTAL_2_ACHIEVEMENTS });
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Portal 2');

    const section = await within(page).findByRole('region', { name: 'Achievements' });
    expect(within(section).getByText('2 / 4 unlocked')).toBeInTheDocument();
  });

  it('sets unlocked and locked Achievements apart', async () => {
    const { operations } = fakeOperations([PORTAL_2], { 'steam:620': PORTAL_2_ACHIEVEMENTS });
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Portal 2');

    const items = within(await within(page).findByRole('list', { name: 'Achievements of Portal 2' })).getAllByRole('listitem');
    expect(items.map((item) => item.dataset.unlocked)).toEqual(['true', 'true', 'false', 'false']);
    expect(items[0]).not.toHaveTextContent('Locked');
  });

  it('shows an Achievement without a description or an icon without them', async () => {
    const { operations } = fakeOperations([PORTAL_2], { 'steam:620': PORTAL_2_ACHIEVEMENTS });
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Portal 2');

    const items = within(await within(page).findByRole('list', { name: 'Achievements of Portal 2' })).getAllByRole('listitem');
    expect(items[3]).toHaveTextContent(/^PartnerLocked$/);
    expect(items[3].querySelector('img')).toBeNull();
  });

  it('says that a Steam Game without Achievements has none', async () => {
    const { operations } = fakeOperations([PORTAL_2], { 'steam:620': { state: 'noAchievements' } });
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Portal 2');

    expect(await within(page).findByText('Portal 2 has no achievements.')).toBeInTheDocument();
    expect(within(page).queryByRole('list', { name: 'Achievements of Portal 2' })).not.toBeInTheDocument();
  });

  it('says when the Achievements have not been fetched yet', async () => {
    const { operations } = fakeOperations([PORTAL_2], { 'steam:620': { state: 'notFetchedYet' } });
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Portal 2');

    expect(
      await within(page).findByText('The achievements have not been fetched yet. They arrive with one of the next syncs.'),
    ).toBeInTheDocument();
  });

  it('says so when the Achievements could not be loaded', async () => {
    const { operations } = fakeOperations([PORTAL_2], {});
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Portal 2');

    expect(await within(page).findByRole('alert')).toHaveTextContent('The achievements could not be loaded.');
  });

  it('reads the Achievements again after a Sync', async () => {
    const achievements: Record<string, GameAchievements> = { 'steam:620': { state: 'notFetchedYet' } };
    const { operations } = fakeOperations([PORTAL_2], achievements, () => {
      achievements['steam:620'] = PORTAL_2_ACHIEVEMENTS;
    });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetailPage('Portal 2');
    await within(page).findByText(/not been fetched yet/);

    await user.click(screen.getByRole('button', { name: 'Sync now' }));

    expect(await within(page).findByText('2 / 4 unlocked')).toBeInTheDocument();
  });

  it('is in German for a German interface', async () => {
    const { operations } = fakeOperations([PORTAL_2], { 'steam:620': PORTAL_2_ACHIEVEMENTS });
    render(<App operations={operations} lang="de" />);

    const { page } = await openDetailPage('Portal 2', 'Details zu Portal 2');

    const section = await within(page).findByRole('region', { name: 'Trophäen' });
    expect(within(section).getByText('2 / 4 freigeschaltet')).toBeInTheDocument();
    const items = within(within(section).getByRole('list', { name: 'Trophäen von Portal 2' })).getAllByRole('listitem');
    expect(items[0]).toHaveTextContent('Freigeschaltet am 20.11.2026');
    expect(items[2]).toHaveTextContent('Noch nicht freigeschaltet');
  });
});

describe('The Achievements on the detail page of a Local Game', () => {
  it('explain that Local Games have no Achievements', async () => {
    const local = game('local:1', 'Own game', { source: 'local' });
    const { operations } = fakeOperations([local], { 'local:1': { state: 'localGame' } });
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetailPage('Own game');

    expect(await within(page).findByText('Local games have no achievements.')).toBeInTheDocument();
  });

  it('explain it in German for a German interface', async () => {
    const local = game('local:1', 'Eigenes Spiel', { source: 'local' });
    const { operations } = fakeOperations([local], { 'local:1': { state: 'localGame' } });
    render(<App operations={operations} lang="de" />);

    const { page } = await openDetailPage('Eigenes Spiel', 'Details zu Eigenes Spiel');

    expect(await within(page).findByText('Lokale Spiele haben keine Trophäen.')).toBeInTheDocument();
  });
});
