import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Operations, TrayMenuTexts } from './operations';

/**
 * Fake operations that keep the close-to-tray setting like the core does
 * and let the test close the window as the user's click on its close button
 * would (`closeButton`).
 */
function fakeOperations({ closeToTray = true } = {}) {
  const calls = { windowClosed: 0, closeToTray: [] as boolean[], trayMenus: [] as TrayMenuTexts[] };
  let setting = closeToTray;
  const noticeListeners = new Set<() => void>();
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
    onAutomaticSync: () => () => {},
    filterLibrary: async () => [],
    game: async () => null,
    closeToTray: async () => setting,
    setCloseToTray: async (on) => {
      calls.closeToTray.push(on);
      setting = on;
    },
    closeWindow: async () => {
      calls.windowClosed += 1;
    },
    onTrayNotice: (listener) => {
      noticeListeners.add(listener);
      return () => noticeListeners.delete(listener);
    },
    showTrayMenu: async (texts) => {
      calls.trayMenus.push(texts);
    },
  };
  /** The first close of the window, which the core answers with the Tray notice. */
  const firstClose = () => act(() => noticeListeners.forEach((listener) => listener()));
  return { operations, calls, firstClose };
}

async function openWindowSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: lang === 'en' ? 'Window' : 'Fenster' }) };
}

describe('The Tray', () => {
  it('gets its menu in the interface language', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await waitFor(() => expect(calls.trayMenus).toEqual([{ tooltip: 'NiceLibrary', open: 'Open NiceLibrary', quit: 'Quit' }]));
  });

  it('gets its menu in German', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="de" />);

    await waitFor(() =>
      expect(calls.trayMenus).toEqual([{ tooltip: 'NiceLibrary', open: 'NiceLibrary öffnen', quit: 'Beenden' }]),
    );
  });
});

describe('The Tray notice', () => {
  it('explains on the first close that the app keeps running in the Tray', async () => {
    const { operations, firstClose } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    firstClose();

    const notice = await screen.findByRole('dialog', { name: 'NiceLibrary keeps running' });
    expect(notice).toHaveTextContent('tray');
    expect(notice).toHaveTextContent('Settings');
  });

  it('closes the window once the user has read it', async () => {
    const { operations, calls, firstClose } = fakeOperations();
    const user = userEvent.setup();
    render(<App operations={operations} lang="en" />);
    firstClose();

    await user.click(await screen.findByRole('button', { name: 'OK' }));

    expect(calls.windowClosed).toBe(1);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('explains it in German too', async () => {
    const { operations, firstClose } = fakeOperations();
    render(<App operations={operations} lang="de" />);

    firstClose();

    const notice = await screen.findByRole('dialog', { name: 'NiceLibrary läuft weiter' });
    expect(notice).toHaveTextContent('Infobereich');
    expect(within(notice).getByRole('button', { name: 'OK' })).toBeInTheDocument();
  });

  it('is not there before the window is closed', async () => {
    const { operations, calls } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    await waitFor(() => expect(calls.trayMenus).toHaveLength(1));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

describe('The close-to-tray setting', () => {
  it('is on by default', async () => {
    const { operations } = fakeOperations();
    const { section } = await openWindowSettings(operations);

    expect(await within(section).findByRole('checkbox', { name: 'Keep running in the tray when the window is closed' })).toBeChecked();
  });

  it('turns close-to-tray off and on again', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openWindowSettings(operations);
    const checkbox = await within(section).findByRole('checkbox');

    await user.click(checkbox);
    expect(checkbox).not.toBeChecked();
    await user.click(checkbox);

    expect(checkbox).toBeChecked();
    expect(calls.closeToTray).toEqual([false, true]);
  });

  it('says that closing the window quits the app while it is off', async () => {
    const { operations } = fakeOperations({ closeToTray: false });
    const { section } = await openWindowSettings(operations);

    expect(await within(section).findByRole('checkbox')).not.toBeChecked();
    expect(section).toHaveTextContent('Closing the window quits NiceLibrary');
  });

  it('is named in German', async () => {
    const { operations } = fakeOperations();
    const { section } = await openWindowSettings(operations, 'de');

    expect(
      await within(section).findByRole('checkbox', { name: 'Beim Schließen des Fensters im Infobereich weiterlaufen' }),
    ).toBeChecked();
  });

  it('says so when it cannot be read', async () => {
    const { operations } = fakeOperations();
    operations.closeToTray = () => Promise.reject(new Error('database: locked'));
    const { section } = await openWindowSettings(operations);

    expect(await within(section).findByRole('alert')).toHaveTextContent('The window setting could not be loaded.');
    expect(within(section).queryByRole('checkbox')).not.toBeInTheDocument();
  });

  it('says so when it cannot be saved and keeps showing what is saved', async () => {
    const { operations } = fakeOperations();
    operations.setCloseToTray = () => Promise.reject(new Error('database: locked'));
    const { user, section } = await openWindowSettings(operations);
    const checkbox = await within(section).findByRole('checkbox');

    await user.click(checkbox);

    expect(await within(section).findByRole('alert')).toHaveTextContent('The window setting could not be saved.');
    expect(checkbox).toBeChecked();
  });
});
