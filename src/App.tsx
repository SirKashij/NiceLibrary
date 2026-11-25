import { useEffect, useState, type ReactNode } from 'react';
import { I18nProvider, useT, type Lang, type MessageKey } from './i18n';
import type { Game, Operations } from './operations';

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
        {view === 'settings' && <PlaceholderPage title="nav.settings" note="settings.empty" />}
      </main>
    </div>
  );
}

type LibraryState = { state: 'loading' } | { state: 'ready'; games: Game[] } | { state: 'failed' };

function LibraryView({ operations }: { operations: Operations }) {
  const t = useT();
  const [libraryState, setLibraryState] = useState<LibraryState>({ state: 'loading' });

  useEffect(() => {
    let active = true;
    operations.library().then(
      (games) => active && setLibraryState({ state: 'ready', games }),
      () => active && setLibraryState({ state: 'failed' }),
    );
    return () => {
      active = false;
    };
  }, [operations]);

  return (
    <Page title="nav.library">
      {libraryState.state === 'ready' && libraryState.games.length === 0 && (
        <p className="note">{t('library.empty')}</p>
      )}
      {libraryState.state === 'failed' && (
        <p className="note" role="alert">
          {t('library.loadFailed')}
        </p>
      )}
    </Page>
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
