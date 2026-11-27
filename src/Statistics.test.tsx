import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Lang } from './i18n';
import type { Game, Operations, Statistics, SyncOutcome } from './operations';

/** 2026-11-25 12:00:00 UTC, in seconds since the Unix epoch. */
const NOW = 1_795_608_000;
/** A week, in seconds. */
const WEEK = 7 * 24 * 60 * 60;

/** The last eight weeks up to NOW, oldest first, with the minutes played in each. */
function weeks(...minutes: number[]): Statistics['weeks'] {
  return minutes.map((played, n) => ({
    startsAt: NOW - (8 - n) * WEEK,
    endsAt: NOW - (7 - n) * WEEK,
    minutes: played,
  }));
}

function game(id: string, title: string, playtimeMinutes: number): Game {
  const source = id.startsWith('local:') ? 'local' : 'steam';
  return { id, title, source, installed: true, playtimeMinutes, lastPlayed: null, status: 'unplayed', favorite: false, progress: null };
}

const HADES = game('local:2', 'Hades', 1_800);
const PORTAL_2 = game('steam:620', 'Portal 2', 1_260);
const COUNTER_STRIKE = game('steam:10', 'Counter-Strike', 600);
const CELESTE = game('local:1', 'Celeste', 120);
const PORTAL = game('steam:400', 'Portal', 95);

/** The Statistics of a Library without Games, as the core answers them. */
const NOTHING: Statistics = {
  games: 0,
  playtimeMinutes: 0,
  achievementsUnlocked: 0,
  achievementsTotal: 0,
  achievementShare: null,
  completeGames: 0,
  gamesByStatus: { unplayed: 0, playing: 0, completed: 0, dropped: 0 },
  weeks: weeks(0, 0, 0, 0, 0, 0, 0, 0),
  mostPlayed: [],
};

/** A Library of 12 Games, 23 hours played, 3 of 5 Achievements unlocked. */
const SOME: Statistics = {
  games: 12,
  playtimeMinutes: 1_380,
  achievementsUnlocked: 3,
  achievementsTotal: 5,
  achievementShare: 60,
  completeGames: 2,
  gamesByStatus: { unplayed: 7, playing: 2, completed: 2, dropped: 1 },
  weeks: weeks(0, 90, 0, 0, 30, 600, 45, 150),
  mostPlayed: [HADES, PORTAL_2, COUNTER_STRIKE, CELESTE, PORTAL],
};

/**
 * Fake operations whose `statistics` answers what `answers` gives, one after
 * another, the last one again after that. A Sync succeeds when asked.
 * Calls of `statistics` are counted.
 */
function fakeOperations(answers: (() => Promise<Statistics>)[]) {
  const calls = { statistics: 0 };
  const synced: SyncOutcome = { outcome: 'synced', at: 1_795_608_000 };
  const operations: Operations = {
    library: async () => [],
    readInstalledGames: async () => [],
    steamInstallation: async () => ({ state: 'notFound' }),
    coverHue: async () => null,
    install: async () => ({ outcome: 'started' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    exportData: async () => ({ outcome: 'cancelled' }),
    language: async () => 'system',
    chooseLanguage: async () => {},
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    steamWebApiKeyStored: async () => true,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    openLogFolder: async () => {},
    sync: async () => synced,
    lastSync: async () => null,
    theme: async () => 'system',
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
    onAutomaticSync: () => () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    onSession: () => () => {},
    game: async (id) => SOME.mostPlayed.find((game) => game.id === id) ?? null,
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launch: async () => ({ outcome: 'launched' }),
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    statistics: () => {
      const answer = answers[Math.min(calls.statistics, answers.length - 1)];
      calls.statistics += 1;
      return answer();
    },
  };
  return { operations, calls };
}

/** Opens the Statistics page from the sidebar and answers it. */
async function openStatistics(operations: Operations, lang: Lang = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Statistics' : 'Statistik' }));
  return { user, page: screen.getByRole('region', { name: lang === 'en' ? 'Statistics' : 'Statistik' }) };
}

/** The text of `element` with its spaces as one plain space each. */
const text = (element: Element | null | undefined) => element?.textContent?.replace(/\s+/g, ' ');

/** The figures of a description list, by their terms. */
function figures(list: HTMLElement) {
  return Object.fromEntries([...list.querySelectorAll('dt')].map((term) => [text(term), text(term.nextElementSibling)]));
}

describe('Statistics', () => {
  it('shows the totals of the Library as the core answers them', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations);

    const totals = await within(page).findByLabelText('Totals');
    expect(figures(totals)).toEqual({
      'Total playtime': '23 h',
      'Achievements unlocked': '3 of 5 (60%)',
      'Games at 100% progress': '2',
    });
  });

  it('shows how many Games have each Status', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations);

    const byStatus = await within(page).findByLabelText('Games by status');
    expect(figures(byStatus)).toEqual({ Unplayed: '7', Playing: '2', Completed: '2', Dropped: '1' });
  });

  it('says there are no Achievements yet rather than a share of none', async () => {
    const { operations } = fakeOperations([async () => ({ ...SOME, achievementsTotal: 0, achievementsUnlocked: 0, achievementShare: null })]);
    const { page } = await openStatistics(operations);

    const totals = await within(page).findByLabelText('Totals');
    expect(figures(totals)['Achievements unlocked']).toBe('None yet');
  });

  it('explains the empty page before the Library holds any Game', async () => {
    const { operations } = fakeOperations([async () => NOTHING]);
    const { page } = await openStatistics(operations);

    expect(
      await within(page).findByText('No statistics yet. Once your library holds games, their figures show up here.'),
    ).toBeInTheDocument();
    expect(within(page).queryByLabelText('Totals')).not.toBeInTheDocument();
  });

  it('says so when the Statistics could not be loaded', async () => {
    const { operations } = fakeOperations([() => Promise.reject(new Error('database: locked'))]);
    const { page } = await openStatistics(operations);

    expect(await within(page).findByRole('alert')).toHaveTextContent('The statistics could not be loaded.');
  });

  it('asks the core again after a Sync, which may change the figures', async () => {
    const { operations } = fakeOperations([async () => NOTHING, async () => SOME]);
    const { user, page } = await openStatistics(operations);
    await within(page).findByText(/No statistics yet/);

    await user.click(screen.getByRole('button', { name: 'Sync now' }));

    const totals = await within(page).findByLabelText('Totals');
    expect(figures(totals)['Games at 100% progress']).toBe('2');
  });

  it('asks the core each time the page is opened', async () => {
    const { operations, calls } = fakeOperations([async () => SOME]);
    const { user, page } = await openStatistics(operations);
    await within(page).findByLabelText('Totals');

    await user.click(screen.getByRole('button', { name: 'Library' }));
    await user.click(screen.getByRole('button', { name: 'Statistics' }));

    await screen.findByLabelText('Totals');
    expect(calls.statistics).toBe(2);
  });

  it('shows the figures in German', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations, 'de');

    const totals = await within(page).findByLabelText('Gesamt');
    expect(figures(totals)).toEqual({
      'Gesamte Spielzeit': '23 Std.',
      'Freigeschaltete Trophäen': '3 von 5 (60 %)',
      'Spiele mit 100 % Fortschritt': '2',
    });
    const byStatus = within(page).getByLabelText('Spiele nach Status');
    expect(figures(byStatus)).toEqual({ Ungespielt: '7', 'Wird gespielt': '2', Durchgespielt: '2', Abgebrochen: '1' });
  });
});

/** The texts of the items of `list`, in order. */
const items = (list: HTMLElement) => within(list).getAllByRole('listitem').map((item) => text(item)?.trim());

describe('Playtime per week on the Statistics page', () => {
  it('labels each bar of the last eight weeks with its Playtime and its week, oldest first', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations);

    const chart = await within(page).findByRole('list', { name: 'Playtime per week' });
    expect(items(chart)).toEqual([
      '0 min 7 weeks ago',
      '1.5 h 6 weeks ago',
      '0 min 5 weeks ago',
      '0 min 4 weeks ago',
      '30 min 3 weeks ago',
      '10 h 2 weeks ago',
      '45 min 1 week ago',
      '2.5 h This week',
    ]);
  });

  it('draws each bar as high as its share of the longest week', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations);

    const chart = await within(page).findByRole('list', { name: 'Playtime per week' });
    const bars = [...chart.querySelectorAll<HTMLElement>('.week-bar')].map((bar) => bar.style.height);
    expect(bars).toEqual(['0%', '15%', '0%', '0%', '5%', '100%', '7.5%', '25%']);
  });

  it('names the days of each week when pointed at', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations);

    const chart = await within(page).findByRole('list', { name: 'Playtime per week' });
    expect(within(chart).getAllByRole('listitem')[7]).toHaveAttribute('title', 'Nov 18, 2026 to Nov 25, 2026: 2.5 h');
  });

  it('states that it only counts the time the app recorded', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations);

    const chart = await within(page).findByRole('region', { name: 'Playtime per week' });
    expect(chart).toHaveTextContent('Counts only the time this app recorded while a game ran.');
  });

  it('labels the weeks in German', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations, 'de');

    const chart = await within(page).findByRole('list', { name: 'Spielzeit pro Woche' });
    expect(items(chart).slice(5)).toEqual(['10 Std. vor 2 Wochen', '45 Min. vor 1 Woche', '2,5 Std. Diese Woche']);
    expect(within(page).getByRole('region', { name: 'Spielzeit pro Woche' })).toHaveTextContent(
      'Zählt nur die Zeit, die diese App aufgezeichnet hat, während ein Spiel lief.',
    );
  });
});

describe('The most played Games on the Statistics page', () => {
  it('lists the five most played Games with their Playtime, most played first', async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { page } = await openStatistics(operations);

    const list = await within(page).findByRole('list', { name: 'Most played' });
    expect(items(list)).toEqual(['1 Hades 30 h', '2 Portal 2 21 h', '3 Counter-Strike 10 h', '4 Celeste 2 h', '5 Portal 1.6 h']);
  });

  it("opens a most played Game's detail page", async () => {
    const { operations } = fakeOperations([async () => SOME]);
    const { user, page } = await openStatistics(operations);

    const list = await within(page).findByRole('list', { name: 'Most played' });
    await user.click(within(list).getByRole('button', { name: /Portal 2/ }));

    const detail = await screen.findByRole('region', { name: 'Portal 2' });
    expect(within(detail).getByRole('heading', { level: 1, name: 'Portal 2' })).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Statistics' })).not.toBeInTheDocument();

    await user.click(within(detail).getByRole('button', { name: 'Back to the library' }));
    expect(screen.getByRole('button', { name: 'Library' })).toHaveAttribute('aria-current', 'page');
  });

  it('says so while no Game has been played yet', async () => {
    const { operations } = fakeOperations([async () => ({ ...SOME, mostPlayed: [] })]);
    const { page } = await openStatistics(operations);

    const section = await within(page).findByRole('region', { name: 'Most played' });
    expect(section).toHaveTextContent('No game has any playtime yet.');
    expect(within(section).queryByRole('list')).not.toBeInTheDocument();
  });
});
