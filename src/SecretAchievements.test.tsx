import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Achievement, Game, GameAchievements, Operations } from './operations';

/** 2026-11-20 20:15:00 UTC, in seconds since the Unix epoch. */
const LAST_WEEK = 1_795_205_700;

const PORTAL_2: Game = {
  id: 'steam:620',
  title: 'Portal 2',
  source: 'steam',
  installed: true,
  playtimeMinutes: 1_260,
  lastPlayed: null,
  status: 'unplayed',
  favorite: false,
};

function achievement(id: string, name: string, unlockedAt: number | null, details: Partial<Achievement> = {}): Achievement {
  return {
    id,
    name,
    description: `How to get ${name}`,
    unlocked: unlockedAt !== null,
    unlockedAt,
    icon: `${id.toLowerCase()}${unlockedAt === null ? '-grey' : ''}`,
    secret: false,
    ...details,
  };
}

function listed(achievements: Achievement[]): GameAchievements {
  return {
    state: 'listed',
    unlocked: achievements.filter((a) => a.unlocked).length,
    total: achievements.length,
    achievements,
  };
}

/** Portal 2 with one unlocked and one locked Secret Achievement, and one plain locked one. */
const WITH_SECRETS = listed([
  achievement('ACH_FINISH', 'Lunacy', LAST_WEEK, { secret: true }),
  achievement('ACH_SPEED_RUN', 'Speed run', null),
  achievement('ACH_PARTNER', 'Partner', null, { secret: true }),
]);

const WITHOUT_SECRETS = listed([achievement('ACH_FINISH', 'Lunacy', LAST_WEEK), achievement('ACH_SPEED_RUN', 'Speed run', null)]);

/** Fake operations over a Library of Portal 2 alone, whose Achievements are `achievements`. */
function fakeOperations(achievements: GameAchievements): Operations {
  const games = [PORTAL_2];
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'synced', at: LAST_WEEK }),
    lastSync: async () => null,
    onAutomaticSync: () => () => {},
    filterLibrary: async () => games,
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    achievements: async () => achievements,
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
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    language: async () => 'system',
    chooseLanguage: async () => {},
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    limitedMode: async () => ({ outcome: 'off' }),
    openSteamPrivacySettings: async () => {},
    sessions: async () => [],
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    coverHue: async () => null,
    install: async () => ({ outcome: 'started' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
  };
  return operations;
}

async function openAchievements(achievements: GameAchievements, lang: 'en' | 'de' = 'en') {
  render(<App operations={fakeOperations(achievements)} lang={lang} />);
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: lang === 'en' ? 'Details for Portal 2' : 'Details zu Portal 2' }));
  const page = await screen.findByRole('region', { name: 'Portal 2' });
  const list = await within(page).findByRole('list', { name: lang === 'en' ? 'Achievements of Portal 2' : 'Trophäen von Portal 2' });
  return { user, page, items: () => within(list).getAllByRole('listitem') };
}

describe('Secret Achievements on the detail page of a Steam Game', () => {
  it('conceal the name, description and icon of a locked Secret Achievement behind a neutral label', async () => {
    const { page, items } = await openAchievements(WITH_SECRETS);

    const partner = items()[2];
    expect(within(partner).getByRole('heading')).toHaveTextContent('Secret achievement');
    expect(partner).toHaveTextContent('Unlock it to find out what it is.');
    expect(partner).toHaveTextContent('Locked');
    expect(partner.querySelector('img')).toBeNull();
    expect(page).not.toHaveTextContent('Partner');
  });

  it('show an unlocked Secret Achievement and a plain locked one as they are', async () => {
    const { items } = await openAchievements(WITH_SECRETS);

    const [lunacy, speedRun] = items();
    expect(within(lunacy).getByRole('heading')).toHaveTextContent('Lunacy');
    expect(lunacy).toHaveTextContent('How to get Lunacy');
    expect(within(speedRun).getByRole('heading')).toHaveTextContent('Speed run');
    expect(speedRun).toHaveTextContent('How to get Speed run');
  });

  it('show the real text of a locked Secret Achievement once the switch is on, and conceal it again when it is off', async () => {
    const { user, page, items } = await openAchievements(WITH_SECRETS);
    const reveal = within(page).getByRole('switch', { name: 'Show secret achievements' });
    expect(reveal).not.toBeChecked();

    await user.click(reveal);

    expect(reveal).toBeChecked();
    const partner = items()[2];
    expect(within(partner).getByRole('heading')).toHaveTextContent('Partner');
    expect(partner).toHaveTextContent('How to get Partner');
    expect(partner.querySelector('img')).toHaveAttribute('src', 'cover://localhost/steam:620/achievements/ach_partner-grey');

    await user.click(reveal);

    expect(within(items()[2]).getByRole('heading')).toHaveTextContent('Secret achievement');
  });

  it('offer no switch for a Game without Secret Achievements', async () => {
    const { page } = await openAchievements(WITHOUT_SECRETS);

    expect(within(page).queryByRole('switch')).not.toBeInTheDocument();
  });

  it('are concealed in German for a German interface', async () => {
    const { page, items } = await openAchievements(WITH_SECRETS, 'de');

    const partner = items()[2];
    expect(within(partner).getByRole('heading')).toHaveTextContent('Geheime Trophäe');
    expect(partner).toHaveTextContent('Schalte sie frei, um zu sehen, was sie ist.');
    expect(within(page).getByRole('switch', { name: 'Geheime Trophäen zeigen' })).toBeInTheDocument();
  });
});
