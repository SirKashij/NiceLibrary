import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { Operations } from './operations';
import { baseOperations } from './test/operations';

/** Fake operations that count how often the log folder is opened. */
function fakeOperations() {
  const calls = { logFolderOpened: 0 };
  const operations: Operations = {
    ...baseOperations(),
    storeSteamWebApiKey: async () => ({ outcome: 'invalid' }),
    createCollection: async () => ({ outcome: 'emptyName' }),
    renameCollection: async () => ({ outcome: 'emptyName' }),
    openLogFolder: async () => {
      calls.logFolderOpened += 1;
    },
  };
  return { operations, calls };
}

async function openLogSettings(operations: Operations, lang: 'de' | 'en' = 'en') {
  const user = userEvent.setup();
  render(<App operations={operations} lang={lang} />);
  await user.click(screen.getByRole('button', { name: lang === 'en' ? 'Settings' : 'Einstellungen' }));
  return { user, section: screen.getByRole('region', { name: lang === 'en' ? 'Log file' : 'Protokolldatei' }) };
}

describe('The log file for bug reports', () => {
  it('explains that nothing is sent and opens the log folder for the user', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openLogSettings(operations);

    expect(section).toHaveTextContent('Nothing is sent');
    await user.click(within(section).getByRole('button', { name: 'Open log folder' }));

    expect(calls.logFolderOpened).toBe(1);
  });

  it('says so when the log folder cannot be opened', async () => {
    const { operations } = fakeOperations();
    operations.openLogFolder = () => Promise.reject(new Error('no Explorer'));
    const { user, section } = await openLogSettings(operations);

    await user.click(within(section).getByRole('button', { name: 'Open log folder' }));

    expect(await within(section).findByRole('alert')).toHaveTextContent('The log folder could not be opened.');
  });

  it('speaks German', async () => {
    const { operations, calls } = fakeOperations();
    const { user, section } = await openLogSettings(operations, 'de');

    expect(section).toHaveTextContent('Nichts wird gesendet');
    await user.click(within(section).getByRole('button', { name: 'Protokollordner öffnen' }));

    expect(calls.logFolderOpened).toBe(1);
  });
});
