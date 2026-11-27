// The page of the Unreadable Database: the database could not be opened, so the app shows only why and
// where it lies. It offers no repair; the file stays as it is.
import { useLayoutEffect, useState } from 'react';
import { I18nProvider, useT, type Lang, type MessageKey } from './i18n';
import type { Database, Operations, UnreadableReason } from './operations';
import { SystemTheme } from './theme';

const REASONS: Record<UnreadableReason, MessageKey> = {
  newerVersion: 'unreadable.newerVersion',
  damaged: 'unreadable.damaged',
  folderNotWritable: 'unreadable.folderNotWritable',
};

/**
 * Whether the store is open, asked once before the interface starts, so the Library never shows for a
 * database that cannot be opened. A failed answer counts as open, so the Library shows its own errors.
 */
export function startingDatabase(operations: Operations): Promise<Database> {
  return operations.database().catch((): Database => ({ state: 'open' }));
}

/**
 * The whole interface while the database cannot be opened. Language and theme follow the system, as the
 * user's choices live in that database.
 */
export function UnreadableDatabasePage({
  operations,
  lang,
  reason,
  path,
}: {
  operations: Operations;
  lang: Lang;
  reason: UnreadableReason;
  path: string;
}) {
  useLayoutEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);
  return (
    <I18nProvider lang={lang}>
      <SystemTheme>
        <Explanation operations={operations} reason={reason} path={path} />
      </SystemTheme>
    </I18nProvider>
  );
}

function Explanation({ operations, reason, path }: { operations: Operations; reason: UnreadableReason; path: string }) {
  const t = useT();
  const [failed, setFailed] = useState(false);

  const openFolder = () => {
    setFailed(false);
    operations.openDatabaseFolder().catch(() => setFailed(true));
  };

  return (
    <main className="unreadable-database" aria-labelledby="unreadable-title">
      <h1 id="unreadable-title">{t('unreadable.title')}</h1>
      <p>{t(REASONS[reason])}</p>
      <div className="unreadable-path">
        <span className="muted">{t('unreadable.path')}</span>
        <code>{path}</code>
      </div>
      <p className="note">{t('unreadable.untouched')}</p>
      <div className="settings-actions">
        <button type="button" className="button" onClick={openFolder}>
          {t('unreadable.openFolder')}
        </button>
        <button type="button" className="button" onClick={() => void operations.quit()}>
          {t('unreadable.quit')}
        </button>
      </div>
      {failed && (
        <p className="note" role="alert">
          {t('unreadable.openFolderFailed')}
        </p>
      )}
    </main>
  );
}
