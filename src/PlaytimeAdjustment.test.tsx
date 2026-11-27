import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, Operations } from './operations';
import { baseOperations } from './test/operations';

function localGame(id: number, title: string, details: Partial<Game> = {}): Game {
  return {
    id: `local:${id}`,
    title,
    source: 'local',
    installed: true,
    playtimeMinutes: 0,
    lastPlayed: null,
    status: 'unplayed',
    progress: null,
    favorite: false,
    ...details,
  };
}

const PORTAL_2: Game = {
  id: 'steam:620',
  title: 'Portal 2',
  source: 'steam',
  installed: true,
  playtimeMinutes: 1_260,
  lastPlayed: null,
  status: 'unplayed',
  progress: null,
  favorite: false,
};

/** Celeste, with 45 minutes of Sessions. */
const CELESTE = localGame(1, 'Celeste', { playtimeMinutes: 45 });

/**
 * Fake operations over a Library whose Local Games have `sessions` minutes
 * of Sessions each; the Playtime is that plus the adjustment, never below
 * zero, as the core answers it. Calls are recorded.
 */
function fakeOperations(
  initial: Game[],
  options: { adjustments?: Record<string, number>; failSaving?: boolean; failLoading?: boolean } = {},
) {
  const sessions = new Map(initial.map((game) => [game.id, game.playtimeMinutes ?? 0]));
  const adjustments = new Map(Object.entries(options.adjustments ?? {}));
  const current = (game: Game): Game =>
    game.source === 'local'
      ? { ...game, playtimeMinutes: Math.max(0, (sessions.get(game.id) ?? 0) + (adjustments.get(game.id) ?? 0)) }
      : game;
  const games = () => initial.map(current);
  const calls = { adjusted: [] as [string, number | null][], read: [] as string[] };
  const operations: Operations = {
    ...baseOperations(),
    library: async () => games(),
    readInstalledGames: async () => games(),
    filterLibrary: async () => games(),
    game: async (gameId) => games().find((game) => game.id === gameId) ?? null,
    playtimeAdjustment: async (gameId) => {
      calls.read.push(gameId);
      if (options.failLoading) throw new Error('database: disk I/O error');
      return adjustments.get(gameId) ?? null;
    },
    setPlaytimeAdjustment: async (gameId, minutes) => {
      calls.adjusted.push([gameId, minutes]);
      if (options.failSaving) throw new Error('database: disk I/O error');
      if (minutes === null || minutes === 0) adjustments.delete(gameId);
      else adjustments.set(gameId, minutes);
      return games().find((game) => game.id === gameId)!;
    },
  };
  return { operations, calls };
}

async function openDetail(title: string, label = `Details for ${title}`) {
  const user = userEvent.setup();
  const grid = await screen.findByRole('list', { name: /Games|Spiele/ });
  await user.click(within(grid).getByRole('button', { name: label }));
  const page = await screen.findByRole('region', { name: title });
  return { user, page };
}

describe('The Playtime Adjustment of a Local Game', () => {
  it('says there is none until the person sets one', async () => {
    const { operations } = fakeOperations([CELESTE]);
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetail('Celeste');

    const adjustment = within(page).getByRole('region', { name: 'Playtime adjustment' });
    expect(await within(adjustment).findByText(/^No adjustment\./)).toBeInTheDocument();
  });

  it('adds time to the Playtime and shows the adjustment as such', async () => {
    const { operations, calls } = fakeOperations([CELESTE]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Celeste');
    const adjustment = within(page).getByRole('region', { name: 'Playtime adjustment' });
    await within(adjustment).findByText(/^No adjustment\./);

    await user.type(within(adjustment).getByLabelText('Hours'), '10');
    await user.click(within(adjustment).getByRole('button', { name: 'Save adjustment' }));

    expect(await within(adjustment).findByText('10 h 0 min added to the time from sessions')).toBeInTheDocument();
    expect(calls.adjusted).toEqual([['local:1', 600]]);
    expect(within(page).getByLabelText('About Celeste')).toHaveTextContent('Playtime10.8 h');
  });

  it('removes time from the Playtime', async () => {
    const { operations, calls } = fakeOperations([CELESTE]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Celeste');
    const adjustment = within(page).getByRole('region', { name: 'Playtime adjustment' });
    await within(adjustment).findByText(/^No adjustment\./);

    await user.click(within(adjustment).getByRole('button', { name: 'Remove time' }));
    await user.type(within(adjustment).getByLabelText('Minutes'), '20');
    await user.click(within(adjustment).getByRole('button', { name: 'Save adjustment' }));

    expect(await within(adjustment).findByText('20 min removed from the time from sessions')).toBeInTheDocument();
    expect(calls.adjusted).toEqual([['local:1', -20]]);
    expect(within(page).getByLabelText('About Celeste')).toHaveTextContent('Playtime25 min');
  });

  it('shows the stored adjustment ready to change, and changes it', async () => {
    const { operations, calls } = fakeOperations([CELESTE], { adjustments: { 'local:1': 150 } });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Celeste');
    const adjustment = within(page).getByRole('region', { name: 'Playtime adjustment' });

    expect(await within(adjustment).findByText('2 h 30 min added to the time from sessions')).toBeInTheDocument();
    expect(within(adjustment).getByLabelText('Hours')).toHaveValue(2);
    expect(within(adjustment).getByLabelText('Minutes')).toHaveValue(30);
    expect(within(adjustment).getByRole('button', { name: 'Add time' })).toHaveAttribute('aria-pressed', 'true');

    await user.clear(within(adjustment).getByLabelText('Hours'));
    await user.type(within(adjustment).getByLabelText('Hours'), '3');
    await user.click(within(adjustment).getByRole('button', { name: 'Save adjustment' }));

    expect(await within(adjustment).findByText('3 h 30 min added to the time from sessions')).toBeInTheDocument();
    expect(calls.adjusted).toEqual([['local:1', 210]]);
  });

  it('can be cleared, which leaves the Playtime of the Sessions', async () => {
    const { operations, calls } = fakeOperations([CELESTE], { adjustments: { 'local:1': 120 } });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Celeste');
    const adjustment = within(page).getByRole('region', { name: 'Playtime adjustment' });
    await within(adjustment).findByText('2 h 0 min added to the time from sessions');

    await user.click(within(adjustment).getByRole('button', { name: 'Clear adjustment' }));

    expect(await within(adjustment).findByText(/^No adjustment\./)).toBeInTheDocument();
    expect(calls.adjusted).toEqual([['local:1', null]]);
    expect(within(page).getByLabelText('About Celeste')).toHaveTextContent('Playtime45 min');
    expect(within(adjustment).queryByRole('button', { name: 'Clear adjustment' })).not.toBeInTheDocument();
  });

  it('refuses an amount that is not whole hours and minutes', async () => {
    const { operations, calls } = fakeOperations([CELESTE]);
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Celeste');
    const adjustment = within(page).getByRole('region', { name: 'Playtime adjustment' });
    await within(adjustment).findByText(/^No adjustment\./);

    await user.click(within(adjustment).getByRole('button', { name: 'Save adjustment' }));

    expect(await within(adjustment).findByRole('alert')).toHaveTextContent('Enter the time in whole hours and minutes.');
    expect(calls.adjusted).toEqual([]);
  });

  it('says so when the adjustment cannot be saved', async () => {
    const { operations } = fakeOperations([CELESTE], { failSaving: true });
    render(<App operations={operations} lang="en" />);
    const { user, page } = await openDetail('Celeste');
    const adjustment = within(page).getByRole('region', { name: 'Playtime adjustment' });
    await within(adjustment).findByText(/^No adjustment\./);

    await user.type(within(adjustment).getByLabelText('Hours'), '1');
    await user.click(within(adjustment).getByRole('button', { name: 'Save adjustment' }));

    expect(await within(adjustment).findByRole('alert')).toHaveTextContent(
      'The playtime adjustment of Celeste could not be saved.',
    );
    expect(within(page).getByLabelText('About Celeste')).toHaveTextContent('Playtime45 min');
  });

  it('says so when the adjustment cannot be read', async () => {
    const { operations } = fakeOperations([CELESTE], { failLoading: true });
    render(<App operations={operations} lang="en" />);
    const { page } = await openDetail('Celeste');
    const adjustment = within(page).getByRole('region', { name: 'Playtime adjustment' });

    expect(await within(adjustment).findByRole('alert')).toHaveTextContent(
      'The playtime adjustment of Celeste could not be read.',
    );
  });

  it('is not offered for a Steam Game', async () => {
    const { operations, calls } = fakeOperations([PORTAL_2, CELESTE]);
    render(<App operations={operations} lang="en" />);

    const { page } = await openDetail('Portal 2');

    expect(within(page).queryByRole('region', { name: 'Playtime adjustment' })).not.toBeInTheDocument();
    expect(calls.read).toEqual([]);
  });

  it('is in German for a German interface', async () => {
    const { operations, calls } = fakeOperations([CELESTE], { adjustments: { 'local:1': 90 } });
    render(<App operations={operations} lang="de" />);
    const { user, page } = await openDetail('Celeste', 'Details zu Celeste');
    const adjustment = within(page).getByRole('region', { name: 'Spielzeit-Korrektur' });

    expect(await within(adjustment).findByText('1 Std. 30 Min. zur Zeit aus Spielsitzungen hinzugefügt')).toBeInTheDocument();
    await user.click(within(adjustment).getByRole('button', { name: 'Zeit abziehen' }));
    await user.clear(within(adjustment).getByLabelText('Stunden'));
    await user.clear(within(adjustment).getByLabelText('Minuten'));
    await user.type(within(adjustment).getByLabelText('Minuten'), '10');
    await user.click(within(adjustment).getByRole('button', { name: 'Korrektur speichern' }));

    expect(await within(adjustment).findByText('10 Min. von der Zeit aus Spielsitzungen abgezogen')).toBeInTheDocument();
    expect(calls.adjusted).toEqual([['local:1', -10]]);
    expect(within(adjustment).getByRole('button', { name: 'Korrektur entfernen' })).toBeInTheDocument();
  });
});
