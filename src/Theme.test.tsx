import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import type { Operations, Theme } from './operations';

/** A promise that never settles: an operation still running. */
const pending = <T,>() => new Promise<T>(() => {});

/** Fake operations that keep the theme like the core does: the system's until the user chooses. */
function fakeOperations({ stored = 'system' as Theme } = {}) {
  const calls = { chosen: [] as Theme[] };
  let theme = stored;
  const operations: Operations = {
    library: async () => [],
    readInstalledGames: async () => [],
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'invalid' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    onAutomaticSync: () => () => {},
    filterLibrary: async () => [],
    game: async () => null,
    theme: async () => theme,
    chooseTheme: async (chosen) => {
      calls.chosen.push(chosen);
      theme = chosen;
    },
    collections: async () => [],
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    deleteCollection: async () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
  };
  return { operations, calls };
}

/** The operating system's dark or light setting, as the webview reports it; `set` changes it while the app runs. */
function systemSetTo(initial: 'dark' | 'light') {
  let light = initial === 'light';
  const listeners = new Set<() => void>();
  vi.stubGlobal('matchMedia', (query: string) => {
    expect(query).toBe('(prefers-color-scheme: light)');
    return {
      get matches() {
        return light;
      },
      media: query,
      addEventListener: (_: 'change', listener: () => void) => listeners.add(listener),
      removeEventListener: (_: 'change', listener: () => void) => listeners.delete(listener),
    };
  });
  return {
    set: (to: 'dark' | 'light') => {
      light = to === 'light';
      act(() => listeners.forEach((listener) => listener()));
    },
  };
}

/** The theme the interface shows, as the token file reads it. */
const shownTheme = () => document.documentElement.dataset.theme;

async function openThemeSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  const section = screen.getByRole('region', { name: lang === 'en' ? 'Theme' : 'Design' });
  return { user, section };
}

afterEach(() => {
  vi.unstubAllGlobals();
  delete document.documentElement.dataset.theme;
});

describe('The theme', () => {
  it('follows the system on first start', async () => {
    systemSetTo('light');
    const { operations } = fakeOperations();
    const { section } = await openThemeSettings(operations);

    expect(shownTheme()).toBe('light');
    expect(within(section).getByRole('radio', { name: 'Match system' })).toBeChecked();
  });

  it('follows the system when it changes while the app runs', async () => {
    const system = systemSetTo('dark');
    const { operations } = fakeOperations();
    render(<App operations={operations} lang="en" />);
    expect(shownTheme()).toBe('dark');

    system.set('light');

    expect(shownTheme()).toBe('light');
  });

  it('starts in the theme chosen before, whatever the system is set to', async () => {
    const system = systemSetTo('dark');
    const { operations } = fakeOperations({ stored: 'light' });
    const { section } = await openThemeSettings(operations);

    expect(await within(section).findByRole('radio', { name: 'Light' })).toBeChecked();
    expect(shownTheme()).toBe('light');

    system.set('light');
    system.set('dark');
    expect(shownTheme()).toBe('light');
  });

  it('switches between dark and light at once and keeps the choice', async () => {
    systemSetTo('dark');
    const { operations, calls } = fakeOperations();
    const { user, section } = await openThemeSettings(operations);

    await user.click(within(section).getByRole('radio', { name: 'Light' }));
    expect(shownTheme()).toBe('light');

    await user.click(within(section).getByRole('radio', { name: 'Dark' }));
    expect(shownTheme()).toBe('dark');
    expect(within(section).getByRole('radio', { name: 'Dark' })).toBeChecked();

    expect(calls.chosen).toEqual(['light', 'dark']);
  });

  it('follows the system again once the user chooses so', async () => {
    systemSetTo('light');
    const { operations, calls } = fakeOperations({ stored: 'dark' });
    const { user, section } = await openThemeSettings(operations);
    await within(section).findByRole('radio', { name: 'Dark', checked: true });

    await user.click(within(section).getByRole('radio', { name: 'Match system' }));

    expect(shownTheme()).toBe('light');
    expect(calls.chosen).toEqual(['system']);
  });

  it('keeps a choice made before the stored theme answers', async () => {
    systemSetTo('dark');
    const { operations } = fakeOperations();
    let answer: (theme: Theme) => void = () => {};
    operations.theme = () => new Promise((resolve) => (answer = resolve));
    const { user, section } = await openThemeSettings(operations);

    await user.click(within(section).getByRole('radio', { name: 'Light' }));
    await act(async () => answer('dark'));

    expect(shownTheme()).toBe('light');
  });

  it('says so and goes back when the choice cannot be saved', async () => {
    systemSetTo('dark');
    const { operations } = fakeOperations();
    operations.chooseTheme = () => Promise.reject(new Error('database is locked'));
    const { user, section } = await openThemeSettings(operations);

    await user.click(within(section).getByRole('radio', { name: 'Light' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent('The theme could not be saved.');
    expect(within(section).getByRole('radio', { name: 'Match system' })).toBeChecked();
    expect(shownTheme()).toBe('dark');
  });

  it('follows the system when the stored theme cannot be read', async () => {
    systemSetTo('light');
    const { operations } = fakeOperations();
    operations.theme = () => Promise.reject(new Error('database is locked'));
    const { section } = await openThemeSettings(operations);

    expect(within(section).getByRole('radio', { name: 'Match system' })).toBeChecked();
    expect(shownTheme()).toBe('light');
  });

  it('shows the system theme while the stored theme is still loading', () => {
    systemSetTo('light');
    const { operations } = fakeOperations();
    operations.theme = pending;
    render(<App operations={operations} lang="en" />);

    expect(shownTheme()).toBe('light');
  });

  it('is offered in German', async () => {
    systemSetTo('dark');
    const { operations } = fakeOperations();
    const { section } = await openThemeSettings(operations, 'de');

    const choice = within(section).getByRole('radiogroup', { name: 'Design der Oberfläche' });
    expect(within(choice).getAllByRole('radio').map((radio) => radio.closest('label')?.textContent)).toEqual([
      'Wie im System',
      'Dunkel',
      'Hell',
    ]);
  });

  it('is offered in English', async () => {
    systemSetTo('dark');
    const { operations } = fakeOperations();
    const { section } = await openThemeSettings(operations);

    const choice = within(section).getByRole('radiogroup', { name: 'Interface theme' });
    expect(within(choice).getAllByRole('radio').map((radio) => radio.closest('label')?.textContent)).toEqual([
      'Match system',
      'Dark',
      'Light',
    ]);
  });
});
