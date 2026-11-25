// PROTOTYPE - small shared atoms. Layout is deliberately NOT shared between variants.
import { useMemo, useState } from 'react';
import { Game, Source, Status, TODAY, computeStats, percent } from './data';
import { Lang, T } from './i18n';

export type View = 'library' | 'stats' | 'settings';

export interface VProps {
  games: Game[];
  view: View;
  go: (v: View) => void;
  gameId: string | null;
  openGame: (id: string | null) => void;
  t: T;
  lang: Lang;
  setLang: (l: Lang) => void;
  theme: 'dark' | 'light';
  setTheme: (t: 'dark' | 'light') => void;
  launch: (g: Game) => void;
}

export function fmtTime(min: number, lang: Lang) {
  if (min === 0) return lang === 'de' ? '0 Std.' : '0 h';
  const h = Math.floor(min / 60);
  const m = min % 60;
  if (h === 0) return lang === 'de' ? `${m} Min.` : `${m} min`;
  if (h >= 10 || m === 0) return lang === 'de' ? `${h} Std.` : `${h} h`;
  return lang === 'de' ? `${h} Std. ${m} Min.` : `${h} h ${m} min`;
}

export function fmtDate(iso: string, lang: Lang) {
  return new Date(iso).toLocaleDateString(lang === 'de' ? 'de-DE' : 'en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
}

export function fmtRel(iso: string | null, lang: Lang, t: T) {
  if (!iso) return t('never');
  const days = Math.round((TODAY.getTime() - new Date(iso).getTime()) / 86400000);
  if (days <= 0) return t('today');
  if (days === 1) return t('yesterday');
  if (days < 30) return lang === 'de' ? `vor ${days} Tagen` : `${days} days ago`;
  return fmtDate(iso, lang);
}

export function fmtNum(n: number, lang: Lang) {
  return n.toLocaleString(lang === 'de' ? 'de-DE' : 'en-GB');
}

const ICONS: Record<string, string> = {
  library: 'M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z',
  stats: 'M5 20V10M12 20V4M19 20v-7',
  settings: 'M4 7h10M18 7h2M4 17h2M10 17h10M16 5v4M8 15v4',
  play: 'M7 4.5v15l12-7.5z',
  search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4',
  star: 'M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.9L12 16.9l-5.2 2.8 1-5.9L3.5 9.7l5.9-.8z',
  back: 'M15 5l-7 7 7 7',
  close: 'M6 6l12 12M18 6L6 18',
  list: 'M4 6h16M4 12h16M4 18h16',
  grid: 'M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z',
  plus: 'M12 5v14M5 12h14',
  sync: 'M4 12a8 8 0 0 1 14-5l2 2M20 12a8 8 0 0 1-14 5l-2-2M20 4v5h-5M4 20v-5h5',
  download: 'M12 4v11M7 11l5 5 5-5M5 20h14',
  trophy: 'M8 4h8v5a4 4 0 0 1-8 0zM8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20h8',
};

export function Icon({ name, size = 18, fill = false }: { name: string; size?: number; fill?: boolean }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill={fill ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={ICONS[name]} />
    </svg>
  );
}

/** Generated placeholder cover. No real artwork anywhere in the prototype. */
export function Cover({ game, className = '' }: { game: Game; className?: string }) {
  const initials = game.title
    .split(/\s+/)
    .filter((w) => /[A-Za-zÄÖÜ0-9]/.test(w[0]))
    .slice(0, 2)
    .map((w) => w[0].toUpperCase())
    .join('');
  const h = game.hue;
  return (
    <div
      className={`cover ${className}`}
      style={{ background: `linear-gradient(155deg, hsl(${h} 62% 44%), hsl(${(h + 45) % 360} 58% 20%))` }}
    >
      <span className="cover-orb" style={{ background: `hsl(${(h + 20) % 360} 80% 66% / 0.35)` }} />
      <span className="cover-initials">{initials}</span>
      <span className="cover-title">{game.title}</span>
    </div>
  );
}

export function Bar({ value }: { value: number | null }) {
  return (
    <div className="bar" role="img" aria-label={value === null ? '-' : `${value} %`}>
      <span style={{ width: `${value ?? 0}%` }} className={value === 100 ? 'full' : ''} />
    </div>
  );
}

export function Ring({ value, size = 72 }: { value: number | null; size?: number }) {
  const r = size / 2 - 6;
  const c = 2 * Math.PI * r;
  return (
    <div className="ring" style={{ width: size, height: size }}>
      <svg width={size} height={size}>
        <circle cx={size / 2} cy={size / 2} r={r} className="ring-bg" />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          className={`ring-fg ${value === 100 ? 'full' : ''}`}
          strokeDasharray={c}
          strokeDashoffset={c * (1 - (value ?? 0) / 100)}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
      </svg>
      <b>{value === null ? '–' : `${value} %`}</b>
    </div>
  );
}

export function StatusBadge({ status, t }: { status: Status; t: T }) {
  return <span className={`badge st-${status}`}>{t(status)}</span>;
}

export function SourceTag({ source, t }: { source: Source; t: T }) {
  return <span className={`tag src-${source}`}>{t(source)}</span>;
}

export function PlayButton({ game, t, launch, big = false, label }: { game: Game; t: T; launch: (g: Game) => void; big?: boolean; label?: string }) {
  return (
    <button className={`btn primary ${big ? 'big' : ''}`} onClick={(e) => { e.stopPropagation(); launch(game); }}>
      <Icon name={game.installed ? 'play' : 'download'} fill={game.installed} size={big ? 20 : 16} />
      {label ?? t(game.installed ? 'play' : 'install')}
    </button>
  );
}

export function AchievementList({ game, t, lang }: { game: Game; t: T; lang: Lang }) {
  const [reveal, setReveal] = useState(false);
  if (game.achievements.length === 0) return <p className="empty">{t('noAch')}</p>;
  const sorted = [...game.achievements].sort((a, b) => Number(b.unlocked) - Number(a.unlocked) || b.rarity - a.rarity);
  const hasHidden = sorted.some((a) => a.hidden);
  return (
    <div className="ach">
      {hasHidden && (
        <label className="check ach-reveal">
          <input type="checkbox" checked={reveal} onChange={(e) => setReveal(e.target.checked)} /> {t('reveal')}
        </label>
      )}
      <ul>
        {sorted.map((a) => {
          const masked = a.hidden && !reveal;
          return (
            <li key={a.id} className={a.unlocked ? 'on' : 'off'}>
              <span className="ach-icon"><Icon name="trophy" size={18} /></span>
              <span className="ach-text">
                <b>{masked ? t('hiddenAch') : a.name}</b>
                <small>{masked ? t('hiddenAchDesc') : a.desc}</small>
              </span>
              <span className="ach-meta">
                <b>{a.rarity.toLocaleString(lang === 'de' ? 'de-DE' : 'en-GB')} %</b>
                <small>{a.unlocked && a.date ? fmtDate(a.date, lang) : t('locked')}</small>
              </span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

export function SessionList({ game, t, lang }: { game: Game; t: T; lang: Lang }) {
  if (game.sessions.length === 0) return <p className="empty">{t('noSessions')}</p>;
  const max = Math.max(...game.sessions.map((s) => s.minutes));
  return (
    <ul className="sess">
      {game.sessions.map((s, i) => (
        <li key={i}>
          <span>{fmtDate(s.date, lang)}</span>
          <span className="sess-bar"><i style={{ width: `${(s.minutes / max) * 100}%` }} /></span>
          <b>{fmtTime(s.minutes, lang)}</b>
        </li>
      ))}
    </ul>
  );
}

export function WeekChart({ games, t, lang, height = 160 }: { games: Game[]; t: T; lang: Lang; height?: number }) {
  const { weeks } = computeStats(games);
  const max = Math.max(...weeks.map((w) => w.minutes), 1);
  return (
    <div className="weeks" style={{ height }}>
      {weeks.map((w) => (
        <div key={w.index} className="week">
          <small>{Math.round(w.minutes / 60)} h</small>
          <span style={{ height: `${(w.minutes / max) * 100}%` }} className={w.index === 0 ? 'now' : ''} />
          <em>{w.index === 0 ? t('thisWeek') : t('weeksAgo').replace('{n}', String(w.index))}</em>
        </div>
      ))}
    </div>
  );
}

function Toggle({ label, on = true }: { label: string; on?: boolean }) {
  const [v, setV] = useState(on);
  return (
    <label className="row toggle">
      <span>{label}</span>
      <input type="checkbox" checked={v} onChange={(e) => setV(e.target.checked)} />
    </label>
  );
}

/** Settings content as sections; each variant decides how to lay the sections out. */
export function SettingsSections({ t, lang, setLang, theme, setTheme }: Pick<VProps, 't' | 'lang' | 'setLang' | 'theme' | 'setTheme'>) {
  return (
    <>
      <section className="set" id="set-steam">
        <h3>{t('steamSection')}</h3>
        <div className="row"><span>{t('account')}</span><b>spieler_eins</b></div>
        <div className="row"><span>{t('apiKey')}</span><span className="row-end"><small>{t('apiKeySet')}</small><button className="btn">{t('change')}</button></span></div>
        <div className="row"><span><small>{t('lastSync')}</small></span><button className="btn"><Icon name="sync" size={15} />{t('syncNow')}</button></div>
      </section>
      <section className="set" id="set-tracking">
        <h3>{t('tracking')}</h3>
        <Toggle label={t('autostart')} />
        <Toggle label={t('closeToTray')} />
      </section>
      <section className="set" id="set-appearance">
        <h3>{t('appearance')}</h3>
        <div className="row">
          <span>{t('theme')}</span>
          <span className="seg">
            <button className={theme === 'dark' ? 'on' : ''} onClick={() => setTheme('dark')}>{t('dark')}</button>
            <button className={theme === 'light' ? 'on' : ''} onClick={() => setTheme('light')}>{t('light')}</button>
          </span>
        </div>
        <div className="row">
          <span>{t('language')}</span>
          <span className="seg">
            <button className={lang === 'de' ? 'on' : ''} onClick={() => setLang('de')}>Deutsch</button>
            <button className={lang === 'en' ? 'on' : ''} onClick={() => setLang('en')}>English</button>
          </span>
        </div>
      </section>
      <section className="set" id="set-updates">
        <h3>{t('updates')}</h3>
        <Toggle label={t('checkUpdates')} />
      </section>
      <section className="set" id="set-about">
        <h3>{t('about')}</h3>
        <p className="about">{t('aboutText')}</p>
      </section>
    </>
  );
}

export type SortKey = 'recent' | 'name' | 'time' | 'progress';

/** Filter/sort state. Controls are rendered differently per variant. */
export function useLibrary(games: Game[], initialMode: 'grid' | 'list' = 'grid') {
  const [query, setQuery] = useState('');
  const [source, setSource] = useState<'all' | Source>('all');
  const [status, setStatus] = useState<'all' | Status>('all');
  const [onlyInstalled, setOnlyInstalled] = useState(false);
  const [onlyFav, setOnlyFav] = useState(false);
  const [collection, setCollection] = useState<string | null>(null);
  const [sort, setSort] = useState<SortKey>('recent');
  const [mode, setMode] = useState<'grid' | 'list'>(initialMode);

  const result = useMemo(() => {
    const q = query.trim().toLowerCase();
    const out = games.filter(
      (g) =>
        (!q || g.title.toLowerCase().includes(q)) &&
        (source === 'all' || g.source === source) &&
        (status === 'all' || g.status === status) &&
        (!onlyInstalled || g.installed) &&
        (!onlyFav || g.favorite) &&
        (!collection || g.collections.includes(collection)),
    );
    const cmp: Record<SortKey, (a: Game, b: Game) => number> = {
      recent: (a, b) => (b.lastPlayed ?? '').localeCompare(a.lastPlayed ?? ''),
      name: (a, b) => a.title.localeCompare(b.title),
      time: (a, b) => b.minutes - a.minutes,
      progress: (a, b) => (percent(b) ?? -1) - (percent(a) ?? -1),
    };
    return out.sort(cmp[sort]);
  }, [games, query, source, status, onlyInstalled, onlyFav, collection, sort]);

  const reset = () => {
    setQuery(''); setSource('all'); setStatus('all'); setOnlyInstalled(false); setOnlyFav(false); setCollection(null);
  };

  return { result, query, setQuery, source, setSource, status, setStatus, onlyInstalled, setOnlyInstalled, onlyFav, setOnlyFav, collection, setCollection, sort, setSort, mode, setMode, reset };
}

export const STATUSES: Status[] = ['unplayed', 'playing', 'completed', 'dropped'];
export const SORTS: [SortKey, string][] = [['recent', 'sortRecent'], ['name', 'sortName'], ['time', 'sortTime'], ['progress', 'sortProgress']];
