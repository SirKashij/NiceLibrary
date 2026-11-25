// PROTOTYPE - throwaway. Question: "What should the app look like?"
// Three structurally different variants of the whole shell (library, game detail, statistics,
// settings), switchable via ?variant=A|B|C and the floating bar at the bottom.
import { useEffect, useState } from 'react';
import { GAMES, Game } from './data';
import { Lang, makeT } from './i18n';
import { View, VProps } from './shared';
import { PrototypeSwitcher } from './PrototypeSwitcher';
import { VariantA } from './variants/VariantA';
import { VariantB } from './variants/VariantB';
import { VariantC } from './variants/VariantC';

const VARIANTS = [
  { key: 'A', name: 'Seitenleiste + Cover-Raster', C: VariantA },
  { key: 'B', name: 'Schaufenster mit Regalen', C: VariantB },
  { key: 'C', name: 'Kompakte Liste + Detailspalte', C: VariantC },
];

function read() {
  const p = new URLSearchParams(location.search);
  return {
    variant: VARIANTS.some((v) => v.key === p.get('variant')) ? p.get('variant')! : 'A',
    view: (['library', 'stats', 'settings'].includes(p.get('view') ?? '') ? p.get('view') : 'library') as View,
    game: p.get('game'),
    theme: (p.get('theme') === 'light' ? 'light' : 'dark') as 'dark' | 'light',
    lang: (p.get('lang') === 'en' ? 'en' : 'de') as Lang,
  };
}

export function App() {
  const [s, setS] = useState(read);
  const [toast, setToast] = useState<string | null>(null);

  const update = (patch: Partial<typeof s>) => {
    setS((prev) => {
      const next = { ...prev, ...patch };
      const p = new URLSearchParams();
      p.set('variant', next.variant);
      p.set('view', next.view);
      if (next.game) p.set('game', next.game);
      p.set('theme', next.theme);
      p.set('lang', next.lang);
      history.replaceState(null, '', `?${p}`);
      return next;
    });
  };

  useEffect(() => {
    document.documentElement.dataset.theme = s.theme;
    document.documentElement.lang = s.lang;
  }, [s.theme, s.lang]);

  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(null), 2600);
    return () => clearTimeout(id);
  }, [toast]);

  const t = makeT(s.lang);
  const props: VProps = {
    games: GAMES,
    view: s.view,
    go: (view) => update({ view }),
    gameId: s.game,
    openGame: (game) => update({ game }),
    t,
    lang: s.lang,
    setLang: (lang) => update({ lang }),
    theme: s.theme,
    setTheme: (theme) => update({ theme }),
    launch: (g: Game) => setToast(`„${g.title}“ ${t('launching')}`),
  };

  const Current = VARIANTS.find((v) => v.key === s.variant)!.C;

  return (
    <>
      <Current key={s.variant} {...props} />
      {toast && <div className="toast" role="status">{toast}</div>}
      {import.meta.env.DEV && (
        <PrototypeSwitcher
          variants={VARIANTS}
          current={s.variant}
          onChange={(variant) => update({ variant })}
          state={`${s.view}${s.game ? ` · ${GAMES.find((g) => g.id === s.game)?.title ?? ''}` : ''}`}
          theme={s.theme}
          lang={s.lang}
          onTheme={() => update({ theme: s.theme === 'dark' ? 'light' : 'dark' })}
          onLang={() => update({ lang: s.lang === 'de' ? 'en' : 'de' })}
        />
      )}
    </>
  );
}
