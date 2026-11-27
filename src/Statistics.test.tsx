import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Lang } from './i18n';
import type { Operations, Statistics, SyncOutcome } from './operations';

/** The Statistics of a Library without Games, as the core answers them. */
const NOTHING: Statistics = {
  games: 0,
  playtimeMinutes: 0,
  achievementsUnlocked: 0,
  achievementsTotal: 0,
  achievementShare: null,
  completeGames: 0,
  gamesByStatus: { unplayed: 0, playing: 0, completed: 0, dropped: 0 },
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
    game: async () => null,
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
