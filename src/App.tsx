import { useEffect, useState, type ReactNode } from 'react';
import { I18nProvider, useT, type Lang, type MessageKey } from './i18n';
import type { Operations } from './operations';

type View = 'library' | 'statistics' | 'settings';

const VIEWS: { view: View; label: MessageKey }[] = [
  { view: 'library', label: 'nav.library' },
  { view: 'statistics', label: 'nav.statistics' },
  { view: 'settings', label: 'nav.settings' },
];

export function App({ operations, lang }: { operations: Operations; lang: Lang }) {
  return (
    <I18nProvider lang={lang}>
      <Shell operations={operations} />
    </I18nProvider>
  );
}

function Shell({ operations }: { operations: Operations }) {
  const t = useT();
  const [view, setView] = useState<View>('library');

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true" />
          {t('app.name')}
        </div>
        <nav aria-label={t('nav.main')}>
          {VIEWS.map((v) => (
            <button
              key={v.view}
              type="button"
              aria-current={view === v.view ? 'page' : undefined}
              onClick={() => setView(v.view)}
            >
              {t(v.label)}
            </button>
          ))}
        </nav>
      </aside>
      <main className="main">
        {view === 'library' && <LibraryView operations={operations} />}
        {view === 'statistics' && <PlaceholderPage title="nav.statistics" note="statistics.empty" />}
        {view === 'settings' && <SettingsView operations={operations} />}
      </main>
    </div>
  );
}

type Loaded<T> = { state: 'loading' } | { state: 'ready'; value: T } | { state: 'failed' };

/** Calls one operation when the view appears and tracks its answer. */
function useOperation<T>(operations: Operations, call: (operations: Operations) => Promise<T>): Loaded<T> {
  const [loaded, setLoaded] = useState<Loaded<T>>({ state: 'loading' });

  useEffect(() => {
    let active = true;
    call(operations).then(
      (value) => active && setLoaded({ state: 'ready', value }),
      () => active && setLoaded({ state: 'failed' }),
    );
    return () => {
      active = false;
    };
    // `call` only names which operation to run and is the same for the life of a view.
  }, [operations]);

  return loaded;
}

function LibraryView({ operations }: { operations: Operations }) {
  const t = useT();
  const library = useOperation(operations, (o) => o.library());

  return (
    <Page title="nav.library">
      {library.state === 'ready' && library.value.length === 0 && <p className="note">{t('library.empty')}</p>}
      {library.state === 'failed' && (
        <p className="note" role="alert">
          {t('library.loadFailed')}
        </p>
      )}
    </Page>
  );
}

function SettingsView({ operations }: { operations: Operations }) {
  return (
    <Page title="nav.settings">
      <SteamSettings operations={operations} />
    </Page>
  );
}

function SteamSettings({ operations }: { operations: Operations }) {
  const t = useT();
  const steam = useOperation(operations, (o) => o.steamInstallation());

  return (
    <section className="settings-section" aria-labelledby="settings-steam">
      <h2 id="settings-steam">{t('settings.steam')}</h2>
      {steam.state === 'failed' && (
        <p className="note" role="alert">
          {t('steam.loadFailed')}
        </p>
      )}
      {steam.state === 'ready' && steam.value.state === 'notFound' && <p className="note">{t('steam.notFound')}</p>}
      {steam.state === 'ready' && steam.value.state === 'found' && (
        <>
          <div className="settings-row">
            <span>{t('steam.found')}</span>
            <span className="muted">{steam.value.path}</span>
          </div>
          {steam.value.accounts.length === 0 ? (
            <p className="note">{t('steam.noAccounts')}</p>
          ) : (
            <ul className="accounts" aria-label={t('steam.accounts')}>
              {steam.value.accounts.map((account) => (
                <li key={account.steamId} className="settings-row">
                  <span>{account.displayName}</span>
                  {account.mostRecent && <span className="muted">{t('steam.mostRecent')}</span>}
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </section>
  );
}

function PlaceholderPage({ title, note }: { title: MessageKey; note: MessageKey }) {
  const t = useT();
  return (
    <Page title={title}>
      <p className="note">{t(note)}</p>
    </Page>
  );
}

function Page({ title, children }: { title: MessageKey; children: ReactNode }) {
  const t = useT();
  return (
    <section className="page">
      <h1>{t(title)}</h1>
      {children}
    </section>
  );
}
