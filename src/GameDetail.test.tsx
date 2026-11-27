import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LaunchOutcome, Operations, SyncOutcome } from './operations';

function steamGame(appId: number, title: string, details: Partial<Game> = {}): Game {
  return {
    id: `steam:${appId}`,
    title,
    source: 'steam',
    installed: true,
    playtimeMinutes: null,
    lastPlayed: null,
    ...details,
  };
}

/** 2026-11-20 20:15:00 UTC, in seconds since the Unix epoch. */
const LAST_WEEK = 1_795_205_700;

const PORTAL = steamGame(400, 'Portal', { installed: false, playtimeMinutes: 0 });
const PORTAL_2 = steamGame(620, 'Portal 2', { playtimeMinutes: 1_260, lastPlayed: LAST_WEEK });

/**
 * Fake operations over a stored Library. `game` answers from the Library as
 * it is when asked; the test can change it. Calls are recorded.
 */
function fakeOperations(
  initial: Game[],
  options: { launch?: (game: Game) => Promise<LaunchOutcome>; sync?: () => Promise<SyncOutcome> } = {},
) {
  let games = initial;
  const calls = { library: 0, launched: [] as string[], game: [] as string[] };
  const operations: Operations = {
    library: async () => {
      calls.library += 1;
      return games;
    },
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: (game) => {
      calls.launched.push(game.id);
      return options.launch ? options.launch(game) : Promise.resolve({ outcome: 'launched' });
    },
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: options.sync ?? (async () => ({ outcome: 'noKey' })),
    lastSync: async () => null,
    theme: async () => 'system',
    chooseTheme: async () => {},
    onAutomaticSync: () => () => {},
    filterLibrary: async () => games,
    game: async (gameId) => {
      calls.game.push(gameId);
      return games.find((game) => game.id === gameId) ?? null;
    },
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
  };
  return { operations, calls, change: (next: Game[]) => (games = next) };
}

async function openFromGrid(title: string, label = `Details for ${title}`) {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: label }));
  return user;
}

describe('The detail page of a Game', () => {
  it('opens from the grid with the large Cover, title, Source, Installed, Playtime and Last Played', async () => {
    const { operations } = fakeOperations([PORTAL, PORTAL_2]);
    render(<App operations={operations} lang="en" />);

    await openFromGrid('Portal 2');

    const page = await screen.findByRole('region', { name: 'Portal 2' });
    expect(within(page).getByRole('heading', { level: 1, name: 'Portal 2' })).toBeInTheDocument();
    expect(within(page).getByRole('img', { name: 'Cover of Portal 2' })).toHaveAttribute(
      'src',
      'cover://localhost/steam:620',
    );
    const facts = within(page).getByLabelText('About Portal 2');
    expect(facts).toHaveTextContent('SourceSteam');
    expect(facts).toHaveTextContent('InstalledYes');
    expect(facts).toHaveTextContent('Playtime21 h');
    expect(facts).toHaveTextContent('Last playedNov 20, 2026');
    expect(screen.queryByRole('list', { name: 'Games' })).not.toBeInTheDocument();
  });

  it('shows a Game that is not Installed and was never played as such, without a play button', async () => {
    const { operations } = fakeOperations([PORTAL, PORTAL_2]);
    render(<App operations={operations} lang="en" />);

    await openFromGrid('Portal');

    const page = await screen.findByRole('region', { name: 'Portal' });
    const facts = within(page).getByLabelText('About Portal');
    expect(facts).toHaveTextContent('InstalledNo');
    expect(facts).toHaveTextContent('PlaytimeNever played');
    expect(facts).toHaveTextContent('Last playedNever');
    expect(within(page).queryByRole('button', { name: /Play/ })).not.toBeInTheDocument();
  });

  it('says that Playtime and Last Played are not known before the first Sync', async () => {
    const { operations } = fakeOperations([steamGame(220, 'Half-Life 2')]);
    render(<App operations={operations} lang="en" />);

    await openFromGrid('Half-Life 2');

    const facts = within(await screen.findByRole('region', { name: 'Half-Life 2' })).getByLabelText('About Half-Life 2');
    expect(facts).toHaveTextContent('PlaytimeNot known yet');
    expect(facts).toHaveTextContent('Last playedNot known yet');
  });

  it('shows the Game as the core answers it now', async () => {
    const { operations, calls, change } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });
    // Since the Library was shown, the Game was uninstalled.
    change([{ ...PORTAL_2, installed: false }]);

    await openFromGrid('Portal 2');

    const page = await screen.findByRole('region', { name: 'Portal 2' });
    expect(await within(page).findByText('No')).toBeInTheDocument();
    expect(within(page).queryByRole('button', { name: 'Play Portal 2' })).not.toBeInTheDocument();
    expect(calls.game).toEqual(['steam:620']);
  });

  it('says so when the Game is no longer in the Library', async () => {
    const { operations, change } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });
    change([]);

    await openFromGrid('Portal 2');

    expect(await screen.findByText('Portal 2 is no longer in your library.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Play Portal 2' })).not.toBeInTheDocument();
  });

  it('reads the Game again after a Sync', async () => {
    const synced = steamGame(220, 'Half-Life 2', { playtimeMinutes: 90, lastPlayed: LAST_WEEK });
    const { operations, change } = fakeOperations([steamGame(220, 'Half-Life 2')], {
      sync: async () => {
        change([synced]);
        return { outcome: 'synced', at: LAST_WEEK };
      },
    });
    render(<App operations={operations} lang="en" />);
    const user = await openFromGrid('Half-Life 2');
    const page = await screen.findByRole('region', { name: 'Half-Life 2' });
    await within(page).findAllByText('Not known yet');

    await user.click(screen.getByRole('button', { name: 'Sync now' }));

    expect(await within(page).findByText('1.5 h')).toBeInTheDocument();
  });

  it('is in German for a German interface', async () => {
    const { operations } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="de" />);

    await openFromGrid('Portal 2', 'Details zu Portal 2');

    const page = await screen.findByRole('region', { name: 'Portal 2' });
    const facts = within(page).getByLabelText('Über Portal 2');
    expect(facts).toHaveTextContent('QuelleSteam');
    expect(facts).toHaveTextContent('InstalliertJa');
    expect(facts).toHaveTextContent('Spielzeit21 Std.');
    expect(facts).toHaveTextContent('Zuletzt gespielt20.11.2026');
    expect(within(page).getByRole('button', { name: 'Portal 2 spielen' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Zurück zur Bibliothek' })).toBeInTheDocument();
  });
});

describe('Going back from the detail page', () => {
  it('restores the Library as it was, without loading it again', async () => {
    const { operations, calls } = fakeOperations([PORTAL, PORTAL_2]);
    render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });
    const loads = calls.library;

    const user = await openFromGrid('Portal 2');
    await user.click(screen.getByRole('button', { name: 'Back to the library' }));

    const grid = screen.getByLabelText('Games');
    expect(within(grid).getAllByRole('heading').map((h) => h.textContent)).toEqual(['Portal', 'Portal 2']);
    expect(screen.queryByRole('region', { name: 'Portal 2' })).not.toBeInTheDocument();
    expect(calls.library).toBe(loads);
  });

  it('restores the scroll position of the Library and its focus', async () => {
    const { operations } = fakeOperations([PORTAL, PORTAL_2]);
    render(<App operations={operations} lang="en" />);
    await screen.findByRole('list', { name: 'Games' });
    const main = screen.getByRole('main');
    main.scrollTop = 600;

    const user = await openFromGrid('Portal 2');
    // The detail page starts at its top.
    expect(main.scrollTop).toBe(0);
    await user.click(screen.getByRole('button', { name: 'Back to the library' }));

    expect(main.scrollTop).toBe(600);
    expect(screen.getByRole('button', { name: 'Details for Portal 2' })).toHaveFocus();
  });
});

describe('The main action on the detail page', () => {
  it('launches an Installed Game as in the Library', async () => {
    const { operations, calls } = fakeOperations([PORTAL_2]);
    render(<App operations={operations} lang="en" />);

    const user = await openFromGrid('Portal 2');
    const page = await screen.findByRole('region', { name: 'Portal 2' });
    await user.click(within(page).getByRole('button', { name: 'Play Portal 2' }));

    expect(calls.launched).toEqual(['steam:620']);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('says clearly when Steam is not available', async () => {
    const { operations } = fakeOperations([PORTAL_2], { launch: async () => ({ outcome: 'steamNotFound' }) });
    render(<App operations={operations} lang="en" />);

    const user = await openFromGrid('Portal 2');
    const page = await screen.findByRole('region', { name: 'Portal 2' });
    await user.click(within(page).getByRole('button', { name: 'Play Portal 2' }));

    expect(await within(page).findByRole('alert')).toHaveTextContent(
      'Portal 2 could not be launched because Steam was not found on this computer.',
    );
  });

  it('says so when the Game could not be launched', async () => {
    const { operations } = fakeOperations([PORTAL_2], { launch: () => Promise.reject(new Error('no')) });
    render(<App operations={operations} lang="en" />);

    const user = await openFromGrid('Portal 2');
    const page = await screen.findByRole('region', { name: 'Portal 2' });
    await user.click(within(page).getByRole('button', { name: 'Play Portal 2' }));

    expect(await within(page).findByRole('alert')).toHaveTextContent('Portal 2 could not be launched.');
  });
});
