import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, Operations, SteamInstallation } from './operations';

const NO_STEAM: SteamInstallation = { state: 'notFound' };

function fakeOperations(games: Game[] = [], steam: SteamInstallation = NO_STEAM): Operations {
  return { library: async () => games, steamInstallation: async () => steam };
}

const TWO_ACCOUNTS: SteamInstallation = {
  state: 'found',
  path: 'C:\\Program Files (x86)\\Steam',
  accounts: [
    { steamId: '76561197960287931', displayName: 'Player Two', mostRecent: true },
    { steamId: '76561197960287930', displayName: 'Player One', mostRecent: false },
  ],
};

async function openSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return screen.getByRole('region', { name: 'Steam' });
}

describe('Steam in the settings', () => {
  it('shows the installation as found and lists each Steam Account, marking the most recent', async () => {
    const steam = await openSettings(fakeOperations([], TWO_ACCOUNTS));

    expect(await within(steam).findByText('Steam installation found')).toBeInTheDocument();
    expect(within(steam).getByText('C:\\Program Files (x86)\\Steam')).toBeInTheDocument();

    const accounts = within(within(steam).getByRole('list', { name: 'Steam accounts on this computer' }))
      .getAllByRole('listitem');
    expect(accounts.map((a) => within(a).getByText(/Player/).textContent)).toEqual(['Player Two', 'Player One']);
    expect(within(accounts[0]).getByText('Most recently used')).toBeInTheDocument();
    expect(within(accounts[1]).queryByText('Most recently used')).not.toBeInTheDocument();
  });

  it('says so when Steam is not installed', async () => {
    const steam = await openSettings(fakeOperations([], NO_STEAM));

    expect(await within(steam).findByText('Steam was not found on this computer.')).toBeInTheDocument();
    expect(within(steam).queryByRole('list')).not.toBeInTheDocument();
  });

  it('says so when no Steam Account has logged in yet', async () => {
    const steam = await openSettings(fakeOperations([], { state: 'found', path: 'C:\\Steam', accounts: [] }));

    expect(await within(steam).findByText('No Steam account has logged in on this computer yet.')).toBeInTheDocument();
  });

  it('says so when the Steam installation cannot be read', async () => {
    const failing: Operations = {
      ...fakeOperations(),
      steamInstallation: () => Promise.reject(new Error('core unavailable')),
    };
    const steam = await openSettings(failing);

    expect(await within(steam).findByRole('alert')).toHaveTextContent('Steam could not be checked.');
  });

  it('speaks German', async () => {
    const steam = await openSettings(fakeOperations([], TWO_ACCOUNTS), 'de');

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
    const failing: Operations = { ...fakeOperations(), library: () => Promise.reject(new Error('core unavailable')) };
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
