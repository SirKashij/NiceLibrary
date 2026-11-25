import { useEffect, useState } from 'react';
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
        {view === 'statistics' && <Page title="nav.statistics" note="statistics.empty" />}
        {view === 'settings' && <Page title="nav.settings" note="settings.empty" />}
      </main>
    </div>
  );
}

type Loaded = { state: 'loading' } | { state: 'ready'; games: Game[] } | { state: 'failed' };

function LibraryView({ operations }: { operations: Operations }) {
  const t = useT();
  const [library, setLibrary] = useState<Loaded>({ state: 'loading' });

  useEffect(() => {
    let current = true;
    operations.library().then(
      (games) => current && setLibrary({ state: 'ready', games }),
      () => current && setLibrary({ state: 'failed' }),
    );
    return () => {
      current = false;
    };
  }, [operations]);

  return (
    <section className="page">
      <h1>{t('nav.library')}</h1>
      {library.state === 'ready' && library.games.length === 0 && <p className="note">{t('library.empty')}</p>}
      {library.state === 'failed' && <p className="note" role="alert">{t('library.loadFailed')}</p>}
    </section>
  );
}

function Page({ title, note }: { title: MessageKey; note: MessageKey }) {
  const t = useT();
  return (
    <section className="page">
      <h1>{t(title)}</h1>
      <p className="note">{t(note)}</p>
    </section>
  );
}
