import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { AddLocalGame, ExecutableChoice, Game, Operations } from './operations';

const CELESTE = 'C:\\Games\\Celeste\\Celeste.exe';

function localGame(id: number, title: string): Game {
  return { id: `local:${id}`, title, source: 'local', installed: true, playtimeMinutes: 0, lastPlayed: null };
}

/**
 * Fake operations over a Library that starts empty. `choice` is what the
 * user does in the file dialog; `add` answers each Local Game to add, by
 * default adding it. `added` records what was asked to be added.
 */
function fakeOperations({
  choice = async () => ({ outcome: 'chosen', executable: CELESTE, title: 'Celeste' }),
  add,
}: {
  choice?: () => Promise<ExecutableChoice>;
  add?: (executable: string, title: string) => Promise<AddLocalGame>;
} = {}) {
  const games: Game[] = [];
  const added: { executable: string; title: string }[] = [];
  const operations: Operations = {
    library: async () => [...games],
    readInstalledGames: async () => [...games],
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    onAutomaticSync: () => () => {},
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    filterLibrary: async () => [...games],
    game: async (gameId) => games.find((game) => game.id === gameId) ?? null,
    chooseLocalGameExecutable: choice,
    addLocalGame: async (executable, title) => {
      added.push({ executable, title });
      if (add) return add(executable, title);
      const game = localGame(games.length + 1, title.trim());
      games.push(game);
      return { outcome: 'added', game };
    },
  };
  return { operations, added };
}

describe('Adding a Local Game', () => {
  it('proposes the derived title, lets the user correct it and shows the Game in the Library', async () => {
    const user = userEvent.setup();
    const { operations, added } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));
    const title = await screen.findByRole('textbox', { name: 'Title' });
    expect(title).toHaveValue('Celeste');
    expect(screen.getByText(CELESTE)).toBeInTheDocument();
    await user.clear(title);
    await user.type(title, 'Celeste Classic');
    await user.click(screen.getByRole('button', { name: 'Add to library' }));

    expect(added).toEqual([{ executable: CELESTE, title: 'Celeste Classic' }]);
    const library = await screen.findByRole('list', { name: 'Games' });
    expect(within(library).getByRole('heading', { name: 'Celeste Classic' })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Celeste Classic was added to your library.');
    expect(screen.queryByRole('textbox', { name: 'Title' })).not.toBeInTheDocument();
  });

  it('shows a Local Game as Installed, with a play button', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));
    await user.click(await screen.findByRole('button', { name: 'Add to library' }));

    const [celeste] = within(await screen.findByRole('list', { name: 'Games' })).getAllByRole('listitem');
    expect(within(celeste).getByRole('button', { name: 'Play Celeste' })).toBeInTheDocument();
    expect(within(celeste).queryByText('Not installed')).not.toBeInTheDocument();
  });

  it('opens the detail page of a Local Game', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));
    await user.click(await screen.findByRole('button', { name: 'Add to library' }));
    const library = await screen.findByRole('list', { name: 'Games' });
    await user.click(within(library).getByRole('button', { name: 'Details for Celeste' }));

    const detail = (await screen.findByRole('heading', { level: 1, name: 'Celeste' })).closest('section')!;
    expect(within(detail).getByText('Local')).toBeInTheDocument();
    expect(within(detail).getByRole('button', { name: 'Play Celeste' })).toBeInTheDocument();
    expect(within(detail).queryByText('Celeste is no longer in your library.')).not.toBeInTheDocument();
  });

  it('adds nothing when the user cancels the file dialog', async () => {
    const user = userEvent.setup();
    const { operations, added } = fakeOperations({ choice: async () => ({ outcome: 'cancelled' }) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));

    expect(await screen.findByRole('button', { name: 'Add a game' })).toBeEnabled();
    expect(screen.queryByRole('textbox', { name: 'Title' })).not.toBeInTheDocument();
    expect(added).toEqual([]);
  });

  it('adds nothing when the user cancels the proposal', async () => {
    const user = userEvent.setup();
    const { operations, added } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));
    await user.click(await screen.findByRole('button', { name: 'Cancel' }));

    expect(screen.queryByRole('textbox', { name: 'Title' })).not.toBeInTheDocument();
    expect(added).toEqual([]);
  });

  it('says so when the chosen executable is already in the Library', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ choice: async () => ({ outcome: 'alreadyAdded', title: 'My Celeste' }) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('This game is already in your library as My Celeste.');
    expect(screen.queryByRole('textbox', { name: 'Title' })).not.toBeInTheDocument();
  });

  it('says so when the core refuses the same executable on adding', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ add: async () => ({ outcome: 'alreadyAdded', title: 'My Celeste' }) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));
    await user.click(await screen.findByRole('button', { name: 'Add to library' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('This game is already in your library as My Celeste.');
  });

  it('asks for a title when it is empty and keeps the proposal open', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ add: async () => ({ outcome: 'noTitle' }) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));
    await user.clear(await screen.findByRole('textbox', { name: 'Title' }));
    await user.click(screen.getByRole('button', { name: 'Add to library' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Enter a title for the game.');
    expect(screen.getByRole('textbox', { name: 'Title' })).toBeInTheDocument();
  });

  it('says so when the executable is gone', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ add: async () => ({ outcome: 'notFound' }) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));
    await user.click(await screen.findByRole('button', { name: 'Add to library' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The file is no longer there. Choose it again.');
  });

  it('says so when the file dialog cannot be opened', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ choice: () => Promise.reject(new Error('file dialog: broken')) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The file dialog could not be opened.');
  });

  it('says so when the Game could not be added', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations({ add: () => Promise.reject(new Error('database: locked')) });
    render(<App operations={operations} lang="en" />);

    await user.click(await screen.findByRole('button', { name: 'Add a game' }));
    await user.click(await screen.findByRole('button', { name: 'Add to library' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The game could not be added.');
  });

  it('speaks German', async () => {
    const user = userEvent.setup();
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="de" />);

    await user.click(await screen.findByRole('button', { name: 'Spiel hinzufügen' }));
    expect(await screen.findByRole('textbox', { name: 'Titel' })).toHaveValue('Celeste');
    await user.click(screen.getByRole('button', { name: 'Zur Bibliothek hinzufügen' }));

    expect(await screen.findByRole('status')).toHaveTextContent('Celeste wurde deiner Bibliothek hinzugefügt.');
  });
});
