import { fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, Operations, SteamInstallation } from './operations';
import { baseOperations } from './test/operations';

const NO_STEAM: SteamInstallation = { state: 'notFound' };

/** Fake operations: `games` is both the stored Library and what reading Steam again finds. */
function fakeOperations(games: Game[] = [], steam: SteamInstallation = NO_STEAM): Operations {
  return {
    ...baseOperations(),
    library: async () => games,
    readInstalledGames: async () => games,
    steamInstallation: async () => steam,
  };
}

/** A promise that never settles: an operation still running. */
const pending = <T,>() => new Promise<T>(() => {});

function steamGame(appId: number, title: string, installed = true): Game {
  return { id: `steam:${appId}`, title, source: 'steam', installed, playtimeMinutes: null, lastPlayed: null, progress: null, status: 'unplayed', favorite: false };
}

function gameTitles() {
  return within(screen.getByRole('list', { name: 'Games' }))
    .getAllByRole('listitem')
    .map((item) => within(item).getByRole('heading').textContent);
}

describe('Installed Steam Games in the Library', () => {
  it('lists each Game by title once Steam has been read', async () => {
    render(
      <App
        operations={{
          ...fakeOperations(),
          readInstalledGames: async () => [steamGame(400, 'Portal'), steamGame(620, 'Portal 2')],
        }}
        lang="en"
      />,
    );

    expect(await screen.findByRole('list', { name: 'Games' })).toBeInTheDocument();
    expect(gameTitles()).toEqual(['Portal', 'Portal 2']);
  });

  it('shows the stored Library at once, while Steam is still being read', async () => {
    render(
      <App
        operations={{ ...fakeOperations([steamGame(620, 'Portal 2')]), readInstalledGames: pending }}
        lang="en"
      />,
    );

    expect(await screen.findByRole('list', { name: 'Games' })).toBeInTheDocument();
    expect(gameTitles()).toEqual(['Portal 2']);
  });

  it('marks a Game that is no longer Installed', async () => {
    render(<App operations={fakeOperations([steamGame(400, 'Portal', false), steamGame(620, 'Portal 2')])} lang="en" />);

    const [portal, portal2] = within(await screen.findByRole('list', { name: 'Games' })).getAllByRole('listitem');
    expect(within(portal).getByText('Not installed')).toBeInTheDocument();
    expect(within(portal2).queryByText('Not installed')).not.toBeInTheDocument();
  });

  it('does not call the Library empty before Steam has been read', async () => {
    render(<App operations={{ ...fakeOperations([]), readInstalledGames: pending }} lang="en" />);

    await screen.findByRole('heading', { level: 1, name: 'Library' });
    await new Promise((r) => setTimeout(r, 0));
    expect(screen.queryByText(/Your library is empty/)).not.toBeInTheDocument();
  });

  it('is a region named after its heading, holding its Games or the note that it is empty', async () => {
    render(<App operations={{ ...fakeOperations([]), limitedMode: async () => ({ outcome: 'noKey' }) }} lang="en" />);

    const library = await screen.findByRole('region', { name: 'Library' });

    expect(await within(library).findByText(/Your library is empty/)).toBeInTheDocument();
  });

  it('keeps the stored Library and says so when Steam cannot be read again', async () => {
    render(
      <App
        operations={{
          ...fakeOperations([steamGame(620, 'Portal 2')]),
          readInstalledGames: () => Promise.reject(new Error('database: disk full')),
        }}
        lang="en"
      />,
    );

    expect(await screen.findByRole('alert')).toHaveTextContent('Your installed games could not be read from Steam.');
    expect(gameTitles()).toEqual(['Portal 2']);
  });

  it('speaks German', async () => {
    render(<App operations={fakeOperations([steamGame(400, 'Portal', false)])} lang="de" />);

    const list = await screen.findByRole('list', { name: 'Spiele' });
    expect(within(list).getByText('Nicht installiert')).toBeInTheDocument();
  });
});

describe('Covers in the Library', () => {
  it('shows each Game as its Cover, loaded from the core', async () => {
    render(<App operations={fakeOperations([steamGame(400, 'Portal'), steamGame(620, 'Portal 2')])} lang="en" />);

    const cover = await screen.findByRole('img', { name: 'Cover of Portal 2' });
    expect(cover.tagName).toBe('IMG');
    expect(cover).toHaveAttribute('src', 'cover://localhost/steam:620');
    expect(screen.getByRole('img', { name: 'Cover of Portal' })).toHaveAttribute('src', 'cover://localhost/steam:400');
  });

  it('shows a neutral placeholder with the title when a Game has no Cover', async () => {
    render(<App operations={fakeOperations([steamGame(3527290, 'Untitled Prototype')])} lang="en" />);

    // The core answers "no Cover" (or cannot fetch one), so the image fails to load.
    fireEvent.error(await screen.findByRole('img', { name: 'Cover of Untitled Prototype' }));

    const placeholder = screen.getByRole('img', { name: 'Cover of Untitled Prototype' });
    expect(placeholder.tagName).not.toBe('IMG');
    expect(placeholder).toHaveTextContent('Untitled Prototype');
  });

  it('keeps the title under each Cover', async () => {
    render(<App operations={fakeOperations([steamGame(620, 'Portal 2')])} lang="en" />);

    expect(await screen.findByRole('heading', { level: 2, name: 'Portal 2' })).toBeInTheDocument();
  });

  it('names Covers in German', async () => {
    render(<App operations={fakeOperations([steamGame(620, 'Portal 2')])} lang="de" />);

    expect(await screen.findByRole('img', { name: 'Cover von Portal 2' })).toBeInTheDocument();
  });
});

const TWO_LOGINS: SteamInstallation = {
  state: 'found',
  path: 'C:\\Program Files (x86)\\Steam',
  logins: [
    { steamId: '76561197960287931', displayName: 'Player Two', mostRecent: true },
    { steamId: '76561197960287930', displayName: 'Player One', mostRecent: false },
  ],
  loginsUnreadable: false,
};

const UNREADABLE_LOGINS: SteamInstallation = {
  state: 'found',
  path: 'C:\\Steam',
  logins: [],
  loginsUnreadable: true,
};

async function openSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return screen.getByRole('region', { name: 'Steam' });
}

describe('Steam in the settings', () => {
  it('shows the installation as found and lists each Steam Login, marking the most recent', async () => {
    const steam = await openSettings(fakeOperations([], TWO_LOGINS));

    expect(await within(steam).findByText('Steam installation found')).toBeInTheDocument();
    expect(within(steam).getByText('C:\\Program Files (x86)\\Steam')).toBeInTheDocument();

    const logins = within(within(steam).getByRole('list', { name: 'Steam accounts on this computer' }))
      .getAllByRole('listitem');
    expect(logins.map((login) => within(login).getByText(/Player/).textContent)).toEqual(['Player Two', 'Player One']);
    expect(within(logins[0]).getByText('Most recently used')).toBeInTheDocument();
    expect(within(logins[1]).queryByText('Most recently used')).not.toBeInTheDocument();
  });

  it('says so when Steam is not installed', async () => {
    const steam = await openSettings(fakeOperations([], NO_STEAM));

    expect(await within(steam).findByText('Steam was not found on this computer.')).toBeInTheDocument();
    expect(within(steam).queryByRole('list')).not.toBeInTheDocument();
  });

  it('says so when there is no Steam Login yet', async () => {
    const steam = await openSettings(fakeOperations([], { state: 'found', path: 'C:\\Steam', logins: [], loginsUnreadable: false }));

    expect(await within(steam).findByText('No Steam account has logged in on this computer yet.')).toBeInTheDocument();
  });

  it('says so when the list of Steam Logins cannot be read, not that there is none', async () => {
    const steam = await openSettings(fakeOperations([], UNREADABLE_LOGINS));

    expect(await within(steam).findByText('Steam installation found')).toBeInTheDocument();
    expect(within(steam).getByText('C:\\Steam')).toBeInTheDocument();
    expect(within(steam).getByText('Steam was found, but its list of accounts could not be read.')).toBeInTheDocument();
    expect(within(steam).queryByText('No Steam account has logged in on this computer yet.')).not.toBeInTheDocument();
  });

  it('says so in German when the list of Steam Logins cannot be read', async () => {
    const steam = await openSettings(fakeOperations([], UNREADABLE_LOGINS), 'de');

    expect(
      await within(steam).findByText('Steam wurde gefunden, aber die Liste der Steam-Konten konnte nicht gelesen werden.'),
    ).toBeInTheDocument();
    expect(
      within(steam).queryByText('Auf diesem Computer hat sich noch kein Steam-Konto angemeldet.'),
    ).not.toBeInTheDocument();
  });

  it('says so when the Steam installation cannot be read', async () => {
    const failing: Operations = {
      ...fakeOperations(),
      steamInstallation: () => Promise.reject(new Error('core unavailable')),
      exportData: async () => ({ outcome: 'cancelled' }),
    };
    const steam = await openSettings(failing);

    expect(await within(steam).findByRole('alert')).toHaveTextContent('Steam could not be checked.');
  });

  it('speaks German', async () => {
    const steam = await openSettings(fakeOperations([], TWO_LOGINS), 'de');

    expect(await within(steam).findByText('Steam-Installation gefunden')).toBeInTheDocument();
    expect(within(steam).getByRole('list', { name: 'Steam-Konten auf diesem Computer' })).toBeInTheDocument();
    expect(within(steam).getByText('Zuletzt verwendet')).toBeInTheDocument();
  });
});

describe('app shell', () => {
  it('shows the sidebar with Library, Statistics and Settings', () => {
    render(<App operations={fakeOperations()} lang="en" />);

    const nav = screen.getByRole('navigation', { name: 'Main' });
    const items = within(nav).getAllByRole('button').map((b) => b.textContent);
    expect(items).toEqual(['Library', 'Statistics', 'Settings']);
  });

  it('opens on the Library', () => {
    render(<App operations={fakeOperations()} lang="en" />);

    expect(screen.getByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Library' })).toHaveAttribute('aria-current', 'page');
  });

  it('explains an empty Library', async () => {
    render(<App operations={fakeOperations([])} lang="en" />);

    expect(await screen.findByText(/Your library is empty/)).toBeInTheDocument();
  });

  it('says so when the Library cannot be loaded', async () => {
    const failing: Operations = {
      ...fakeOperations(),
      library: () => Promise.reject(new Error('core unavailable')),
      readInstalledGames: () => Promise.reject(new Error('core unavailable')),
    };
    render(<App operations={failing} lang="en" />);

    expect(await screen.findByRole('alert')).toHaveTextContent('The library could not be loaded.');
    expect(screen.queryByText(/Your library is empty/)).not.toBeInTheDocument();
  });

  it('switches to Statistics and Settings from the sidebar', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations()} lang="en" />);

    await user.click(screen.getByRole('button', { name: 'Statistics' }));
    expect(screen.getByRole('heading', { level: 1, name: 'Statistics' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Settings' }));
    expect(screen.getByRole('heading', { level: 1, name: 'Settings' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Settings' })).toHaveAttribute('aria-current', 'page');
  });

  it('reaches every sidebar entry with the keyboard', async () => {
    const user = userEvent.setup();
    render(<App operations={fakeOperations()} lang="en" />);

    await user.tab();
    expect(screen.getByRole('button', { name: 'Library' })).toHaveFocus();
    await user.tab();
    expect(screen.getByRole('button', { name: 'Statistics' })).toHaveFocus();
    await user.tab();
    await user.keyboard('{Enter}');
    expect(screen.getByRole('heading', { level: 1, name: 'Settings' })).toBeInTheDocument();
  });

  it('speaks German', async () => {
    render(<App operations={fakeOperations()} lang="de" />);

    const nav = screen.getByRole('navigation', { name: 'Hauptnavigation' });
    expect(within(nav).getAllByRole('button').map((b) => b.textContent)).toEqual([
      'Bibliothek',
      'Statistik',
      'Einstellungen',
    ]);
    expect(await screen.findByText(/Deine Bibliothek ist leer/)).toBeInTheDocument();
  });
});
