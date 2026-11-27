import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Operations, Update, UpdateCheck, UpdateInstall } from './operations';
import { baseOperations } from './test/operations';

const NEWER: Update = { version: '0.2.0', notes: 'Faster covers.' };

/**
 * Fake operations that keep the automatic Update Check setting like the core
 * does and let the test play the automatic check finding an Update (`found`).
 */
function fakeOperations({
  checksForUpdates = true,
  check = { outcome: 'upToDate' } as UpdateCheck,
  install = { outcome: 'started' } as UpdateInstall,
} = {}) {
  const calls = { installed: [] as string[], checks: 0, setting: [] as boolean[] };
  let setting = checksForUpdates;
  const listeners = new Set<(update: Update) => void>();
  const operations: Operations = {
    ...baseOperations(),
    checksForUpdates: async () => setting,
    setCheckForUpdates: async (on) => {
      calls.setting.push(on);
      setting = on;
    },
    checkForUpdate: async () => {
      calls.checks += 1;
      return check;
    },
    installUpdate: async (version) => {
      calls.installed.push(version);
      return install;
    },
    onUpdateFound: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  const found = (update: Update) => act(() => listeners.forEach((listener) => listener(update)));
  return { operations, calls, found };
}

async function openUpdateSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: 'Updates' }) };
}

describe('An Update the automatic check found', () => {
  it('is offered with what is new, and nothing is installed before the user agrees', async () => {
    const { operations, calls, found } = fakeOperations();
    render(<App operations={operations} lang="en" />);

    found(NEWER);

    const offer = await screen.findByRole('dialog', { name: 'Update available' });
    expect(offer).toHaveTextContent('NiceLibrary 0.2.0 is available.');
    expect(offer).toHaveTextContent('Faster covers.');
    expect(calls.installed).toEqual([]);
  });

  it('is installed once the user agrees', async () => {
    const { operations, calls, found } = fakeOperations();
    const user = userEvent.setup();
    render(<App operations={operations} lang="en" />);
    found(NEWER);

    await user.click(await screen.findByRole('button', { name: 'Install and restart' }));

    expect(calls.installed).toEqual(['0.2.0']);
  });

  it('is not installed when the user puts it off', async () => {
    const { operations, calls, found } = fakeOperations();
    const user = userEvent.setup();
    render(<App operations={operations} lang="en" />);
    found(NEWER);

    await user.click(await screen.findByRole('button', { name: 'Later' }));

    expect(screen.queryByRole('dialog', { name: 'Update available' })).not.toBeInTheDocument();
    expect(calls.installed).toEqual([]);
  });

  it('says so when its signature was refused and nothing was installed', async () => {
    const { operations, found } = fakeOperations({ install: { outcome: 'invalidSignature' } });
    const user = userEvent.setup();
    render(<App operations={operations} lang="en" />);
    found(NEWER);

    await user.click(await screen.findByRole('button', { name: 'Install and restart' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The update was not installed: its signature does not prove that it comes from the NiceLibrary project.',
    );
  });

  it('says so when the update could not be downloaded', async () => {
    const { operations, found } = fakeOperations();
    operations.installUpdate = async () => {
      throw new Error('offline');
    };
    const user = userEvent.setup();
    render(<App operations={operations} lang="en" />);
    found(NEWER);

    await user.click(await screen.findByRole('button', { name: 'Install and restart' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The update could not be downloaded or started.');
  });

  it('is offered in German', async () => {
    const { operations, found } = fakeOperations();
    render(<App operations={operations} lang="de" />);

    found(NEWER);

    const offer = await screen.findByRole('dialog', { name: 'Update verfügbar' });
    expect(offer).toHaveTextContent('NiceLibrary 0.2.0 ist verfügbar.');
    expect(within(offer).getByRole('button', { name: 'Installieren und neu starten' })).toBeInTheDocument();
    expect(within(offer).getByRole('button', { name: 'Später' })).toBeInTheDocument();
  });
});

describe('The Update settings', () => {
  it('show the automatic check on, and switch it off', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openUpdateSettings(operations);

    const checkbox = await within(section).findByRole('checkbox', { name: 'Check for updates automatically' });
    expect(checkbox).toBeChecked();

    await user.click(checkbox);

    expect(calls.setting).toEqual([false]);
    await waitFor(() => expect(checkbox).not.toBeChecked());
  });

  it('check by hand and say when the app is up to date', async () => {
    const { operations, calls } = fakeOperations({ checksForUpdates: false });
    const { user, section } = await openUpdateSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Check for updates now' }));

    expect(await within(section).findByText('NiceLibrary is up to date.')).toBeInTheDocument();
    expect(calls.checks).toBe(1);
  });

  it('offer an Update found by hand and install it only once the user agrees', async () => {
    const { operations, calls } = fakeOperations({ check: { outcome: 'available', update: NEWER } });
    const { user, section } = await openUpdateSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Check for updates now' }));

    const offer = await screen.findByRole('dialog', { name: 'Update available' });
    expect(calls.installed).toEqual([]);
    await user.click(within(offer).getByRole('button', { name: 'Install and restart' }));
    expect(calls.installed).toEqual(['0.2.0']);
  });

  it('say so when GitHub could not be asked', async () => {
    const { operations } = fakeOperations();
    operations.checkForUpdate = async () => {
      throw new Error('offline');
    };
    const { user, section } = await openUpdateSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Check for updates now' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent('GitHub could not be asked for updates.');
  });

  it('are in German too', async () => {
    const { operations } = fakeOperations();
    const { section } = await openUpdateSettings(operations, 'de');

    expect(await within(section).findByRole('checkbox', { name: 'Automatisch nach Updates suchen' })).toBeChecked();
    expect(within(section).getByRole('button', { name: 'Jetzt nach Updates suchen' })).toBeInTheDocument();
  });
});
