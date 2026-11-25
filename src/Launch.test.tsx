import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, LaunchOutcome, Operations } from './operations';

function steamGame(appId: number, title: string, installed = true): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed };
}

/** Fake operations over a stored Library; `launch` records each Game it is asked to start. */
function fakeOperations(games: Game[], launch: (game: Game) => Promise<LaunchOutcome>) {
  const launched: string[] = [];
  const operations: Operations = {
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => ({ state: 'notFound' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: (game) => {
      launched.push(game.id);
      return launch(game);
    },
  };
  return { operations, launched };
}

const started = async (): Promise<LaunchOutcome> => ({ outcome: 'started' });

describe('Launching an Installed Steam Game', () => {
  it('launches the Game whose play button is clicked', async () => {
    const user = userEvent.setup();
    const { operations, launched } = fakeOperations([steamGame(400, 'Portal'), steamGame(620, 'Portal 2')], started);
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Play Portal 2' }));

    expect(launched).toEqual(['steam:620']);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('offers no play button for a Game that is not Installed', async () => {
    const { operations } = fakeOperations([steamGame(400, 'Portal', false), steamGame(620, 'Portal 2')], started);
    render(<App operations={operations} lang="en" />);

    const [portal, portal2] = within(await screen.findByRole('list', { name: 'Games' })).getAllByRole('listitem');
    expect(within(portal).queryByRole('button')).not.toBeInTheDocument();
    expect(within(portal2).getByRole('button', { name: 'Play Portal 2' })).toBeInTheDocument();
  });

  it('says clearly when Steam is not available', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([steamGame(620, 'Portal 2')], async () => ({ outcome: 'steamNotFound' }));
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Play Portal 2' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Portal 2 could not be launched because Steam was not found on this computer.',
    );
  });

  it('says so when the Game could not be launched', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([steamGame(620, 'Portal 2')], () =>
      Promise.reject(new Error('Windows could not open steam://rungameid/620')),
    );
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Play Portal 2' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Portal 2 could not be launched.');
  });

  it('clears the message once a later Launch succeeds', async () => {
    const user = userEvent.setup();
    let steamThere = false;
    const { operations } = fakeOperations([steamGame(620, 'Portal 2')], async () =>
      steamThere ? { outcome: 'started' } : { outcome: 'steamNotFound' },
    );
    render(<App operations={operations} lang="en" />);
    const play = await screen.findByRole('button', { name: 'Play Portal 2' });

    await user.click(play);
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    steamThere = true;
    await user.click(play);

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('speaks German', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations([steamGame(620, 'Portal 2')], async () => ({ outcome: 'steamNotFound' }));
    render(<App operations={operations} lang="de" />);

    await user.click(await screen.findByRole('button', { name: 'Portal 2 spielen' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Portal 2 konnte nicht gestartet werden, weil Steam auf diesem Computer nicht gefunden wurde.',
    );
  });
});
