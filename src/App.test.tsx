import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Game, Operations } from './operations';

function fakeOperations(games: Game[] = []): Operations {
  return { library: async () => games };
}

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
    const failing: Operations = { library: () => Promise.reject(new Error('core unavailable')) };
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
