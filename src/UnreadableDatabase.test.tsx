import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import { messages, type Lang } from './i18n';
import type { Operations, UnreadableReason } from './operations';
import { baseOperations } from './test/operations';
import { startingDatabase } from './UnreadableDatabase';

const PATH = 'C:\\Users\\Player\\AppData\\Roaming\\io.github.sirkashij.nicelibrary\\nicelibrary.db';

/**
 * Fake operations of an app whose database could not be opened for `reason`: every operation that needs
 * the store fails, as the desktop shell's do without a store.
 */
function unreadable(reason: UnreadableReason, { folderOpens = true } = {}) {
  const calls = { foldersOpened: 0, quits: 0, storeCalls: [] as string[] };
  const needsStore = new Proxy(baseOperations(), {
    get(target, name: string) {
      const operation = target[name as keyof Operations];
      if (typeof operation !== 'function') return operation;
      return () => {
        calls.storeCalls.push(name);
        return Promise.reject(new Error(`state not managed for ${name}`));
      };
    },
  });
  const operations: Operations = {
    ...needsStore,
    coverUrl: baseOperations().coverUrl,
    achievementIconUrl: baseOperations().achievementIconUrl,
    onAutomaticSync: () => () => {},
    onAutomaticSteamAppListFetch: () => () => {},
    onTrayNotice: () => () => {},
    onSession: () => () => {},
    database: async () => ({ state: 'unreadable', reason, path: PATH }),
    openDatabaseFolder: async () => {
      calls.foldersOpened += 1;
      if (!folderOpens) throw new Error('Explorer did not start');
    },
    quit: async () => {
      calls.quits += 1;
    },
  };
  return { operations, calls };
}

/** Starts the interface as src/main.tsx does: asks whether the store is open, then renders. */
async function start(operations: Operations, lang: Lang) {
  render(<App operations={operations} lang={lang} database={await startingDatabase(operations)} />);
}

describe('Unreadable Database', () => {
  it.each([
    ['newerVersion', 'unreadable.newerVersion'],
    ['damaged', 'unreadable.damaged'],
    ['folderNotWritable', 'unreadable.folderNotWritable'],
  ] as const)('names the reason %s and the full path of the database', async (reason, key) => {
    await start(unreadable(reason).operations, 'en');

    expect(await screen.findByRole('heading', { name: messages.en['unreadable.title'] })).toBeInTheDocument();
    expect(screen.getByText(messages.en[key])).toBeInTheDocument();
    expect(screen.getByText(PATH)).toBeInTheDocument();
  });

  it('never shows the Library, not even empty, and asks nothing of the store', async () => {
    const { operations, calls } = unreadable('damaged');

    await start(operations, 'en');

    await screen.findByRole('heading', { name: messages.en['unreadable.title'] });
    expect(screen.queryByRole('navigation', { name: messages.en['nav.main'] })).not.toBeInTheDocument();
    expect(screen.queryByText(messages.en['nav.library'])).not.toBeInTheDocument();
    expect(calls.storeCalls).toEqual([]);
  });

  it('follows the system language, as the chosen one lives in the database', async () => {
    await start(unreadable('newerVersion').operations, 'de');

    expect(await screen.findByRole('heading', { name: messages.de['unreadable.title'] })).toBeInTheDocument();
    expect(screen.getByText(messages.de['unreadable.newerVersion'])).toBeInTheDocument();
  });

  it('opens the database folder', async () => {
    const { operations, calls } = unreadable('damaged');
    await start(operations, 'en');

    await userEvent.click(await screen.findByRole('button', { name: messages.en['unreadable.openFolder'] }));

    expect(calls.foldersOpened).toBe(1);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('says so when the folder could not be opened', async () => {
    const { operations } = unreadable('damaged', { folderOpens: false });
    await start(operations, 'en');

    await userEvent.click(await screen.findByRole('button', { name: messages.en['unreadable.openFolder'] }));

    expect(await screen.findByRole('alert')).toHaveTextContent(messages.en['unreadable.openFolderFailed']);
  });

  it('quits the app', async () => {
    const { operations, calls } = unreadable('folderNotWritable');
    await start(operations, 'en');

    await userEvent.click(await screen.findByRole('button', { name: messages.en['unreadable.quit'] }));

    expect(calls.quits).toBe(1);
  });
});

describe('an open database', () => {
  it('shows the Library', async () => {
    await start(baseOperations(), 'en');

    expect(await screen.findByRole('navigation', { name: messages.en['nav.main'] })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: messages.en['unreadable.title'] })).not.toBeInTheDocument();
  });

  it('shows the Library when the question itself fails, which then shows its own errors', async () => {
    const operations: Operations = {
      ...baseOperations(),
      database: async () => {
        throw new Error('no answer');
      },
    };

    await start(operations, 'en');

    expect(await screen.findByRole('navigation', { name: messages.en['nav.main'] })).toBeInTheDocument();
  });
});
