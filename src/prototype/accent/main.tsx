// PROTOTYPE, throwaway, never part of the app.
// Question: where and how strongly should the lime of the new nl monogram (#92) show in the interface?
// Five variants on the real App with fake in-memory data, switchable via ?variant= and the bar at the bottom (← / →).
import { StrictMode, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource-variable/bricolage-grotesque';
import '@fontsource-variable/instrument-sans';
import '../../styles/tokens.css';
import '../../styles/app.css';
import { App } from '../../App';
import { systemLang } from '../../i18n';
import type { Game, Operations } from '../../operations';
import { VARIANTS } from './variants';

// ---------- Fake data ----------
const now = Math.floor(Date.now() / 1000);
const day = 86400;
const raw: [string, number, Partial<Game>][] = [
  ['Ember Hollow', 18, { playtimeMinutes: 3920, progress: 100, status: 'completed', favorite: true, lastPlayed: now - 9 * day }],
  ['Neon Drift', 312, { playtimeMinutes: 1240, progress: 64, status: 'playing', lastPlayed: now - day }],
  ['Glasswater', 196, { playtimeMinutes: 610, progress: 38, status: 'playing', favorite: true, lastPlayed: now - 3 * day }],
  ['Iron Meridian', 28, { playtimeMinutes: 0, progress: 0, installed: false }],
  ['Paper Kingdoms', 48, { playtimeMinutes: 2210, progress: 91, status: 'playing', lastPlayed: now - 2 * day }],
  ['Stillwake', 262, { playtimeMinutes: 95, progress: 12, status: 'dropped', lastPlayed: now - 60 * day }],
  ['Orbital Gardens', 140, { playtimeMinutes: 4800, progress: 100, status: 'completed', lastPlayed: now - 30 * day }],
  ['Hollow Lantern', 0, { playtimeMinutes: 0, progress: null, installed: false }],
  ['Copper Saints', 34, { source: 'local', playtimeMinutes: 420, progress: 50, status: 'playing', lastPlayed: now - 5 * day }],
  ['Tidebound', 210, { playtimeMinutes: 1800, progress: 77, lastPlayed: now - 12 * day }],
  ['Fable of Ash', 8, { playtimeMinutes: 0, progress: 0 }],
  ['Quiet Machines', 170, { source: 'local', playtimeMinutes: 60, progress: null }],
  ['Velvet Circuit', 290, { playtimeMinutes: 980, progress: 100, status: 'completed', favorite: true, lastPlayed: now - 20 * day }],
  ['North of Nowhere', 100, { playtimeMinutes: 330, progress: 22, installed: false, lastPlayed: now - 90 * day }],
];
const hues = new Map<string, number>();
const games: Game[] = raw.map(([title, hue, rest], i) => {
  const source = rest.source ?? 'steam';
  const id = source === 'steam' ? `steam:${1000 + i}` : `local:${i}`;
  hues.set(id, hue);
  return { id, title, source, installed: true, playtimeMinutes: null, lastPlayed: null, progress: null, status: 'unplayed', favorite: false, ...rest };
});

// Generated stand-in Covers: a gradient and the title, no game artwork (ADR-0005).
function cover(game: Game): string {
  const h = hues.get(game.id) ?? 0;
  const words = game.title.split(' ');
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 900">
    <defs><linearGradient id="g" x1="0" y1="0" x2="0.4" y2="1"><stop offset="0" stop-color="hsl(${h} 55% 52%)"/><stop offset="1" stop-color="hsl(${(h + 40) % 360} 60% 18%)"/></linearGradient></defs>
    <rect width="600" height="900" fill="url(#g)"/>
    <circle cx="${380 + (h % 120)}" cy="${260 + (h % 90)}" r="${120 + (h % 60)}" fill="hsl(${(h + 180) % 360} 70% 70%)" opacity="0.35"/>
    ${words.map((w, i) => `<text x="44" y="${700 + i * 86 - (words.length - 1) * 86}" font-family="Georgia, serif" font-size="84" font-weight="700" fill="white">${w}</text>`).join('')}
  </svg>`;
  return 'data:image/svg+xml,' + encodeURIComponent(svg);
}

let theme: 'system' | 'dark' | 'light' = 'dark';
const collections = [
  { id: 1, name: 'Couch co-op', gameCount: 3 },
  { id: 2, name: 'Backlog 2026', gameCount: 5 },
];
const membership: Record<number, string[]> = { 1: [games[1].id, games[4].id, games[9].id], 2: games.slice(3, 8).map((g) => g.id) };

function filtered(f: { search?: string; source?: string | null; installedOnly?: boolean; status?: string | null; favoritesOnly?: boolean; collection?: number; sort?: string }) {
  let list = games.filter(
    (g) =>
      (!f.search || g.title.toLowerCase().includes(f.search.toLowerCase())) &&
      (!f.source || g.source === f.source) &&
      (!f.installedOnly || g.installed) &&
      (!f.status || g.status === f.status) &&
      (!f.favoritesOnly || g.favorite) &&
      (f.collection === undefined || membership[f.collection]?.includes(g.id)),
  );
  if (f.sort === 'playtime') list = [...list].sort((a, b) => (b.playtimeMinutes ?? 0) - (a.playtimeMinutes ?? 0));
  else if (f.sort === 'lastPlayed') list = [...list].sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0));
  else list = [...list].sort((a, b) => a.title.localeCompare(b.title));
  return list;
}

const ops: Operations = {
  library: async () => games,
  readInstalledGames: async () => games.filter((g) => g.installed),
  steamInstallation: async () => ({ state: 'found' }) as never,
  steamAccount: async () => '76561190000000000',
  chooseSteamAccount: async () => ({ outcome: 'chosen' }) as never,
  coverUrl: (game) => cover(game),
  launch: async () => ({ outcome: 'launched' }) as never,
  steamWebApiKeyStored: async () => true,
  storeSteamWebApiKey: async () => ({ outcome: 'stored' }) as never,
  removeSteamWebApiKey: async () => {},
  openSteamWebApiKeyPage: async () => {},
  sync: async () => ({ outcome: 'synced', games: games.length }) as never,
  lastSync: async () => now - 3600,
  onAutomaticSync: () => () => {},
  chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }) as never,
  addLocalGame: async () => ({ outcome: 'noTitle' }) as never,
  theme: async () => theme,
  chooseTheme: async (t) => {
    theme = t as typeof theme;
  },
  collections: async () => collections,
  createCollection: async () => ({ outcome: 'named', id: 3 }),
  renameCollection: async (id) => ({ outcome: 'named', id }),
  deleteCollection: async () => {},
  steamAppList: async () => ({ titles: 180000, fetchedAt: now - day, unfinished: false }) as never,
  fetchSteamAppList: async () => ({ outcome: 'noKey' }) as never,
  onAutomaticSteamAppListFetch: () => () => {},
  filterLibrary: async (f) => filtered(f as never),
  game: async (id) => games.find((g) => g.id === id) ?? null,
  gameCollections: async (id) => Object.entries(membership).filter(([, ids]) => ids.includes(id)).map(([c]) => Number(c)),
  addGameToCollection: async () => {},
  removeGameFromCollection: async () => {},
  libraryLayout: async () => 'coverGrid',
  chooseLibraryLayout: async () => {},
  closeToTray: async () => true,
  setCloseToTray: async () => {},
  closeWindow: async () => {},
  onTrayNotice: () => () => {},
  showTrayMenu: async () => {},
  librarySort: async () => 'name' as never,
  chooseLibrarySort: async () => {},
  language: async () => 'system' as never,
  chooseLanguage: async () => {},
  setManualProgress: async () => ({ outcome: 'set' }) as never,
  chooseCoverImage: async () => ({ outcome: 'cancelled' }),
  coverImageChosen: async () => false,
  removeCoverImage: async () => {},
  setStatus: async (id, status) => {
    const g = games.find((x) => x.id === id)!;
    g.status = status;
    return { ...g };
  },
  openLogFolder: async () => {},
  limitedMode: async () => ({ outcome: 'off' }) as never,
  openSteamPrivacySettings: async () => {},
  startsWithWindows: async () => true,
  setStartWithWindows: async () => {},
  onSession: () => () => {},
  playtimeAdjustment: async () => null,
  setPlaytimeAdjustment: async (id) => ({ ...games.find((g) => g.id === id)! }),
  sessions: async () => [
    { startedAt: now - day - 7200, seconds: 5400, running: false },
    { startedAt: now - 3 * day, seconds: 2700, running: false },
  ] as never,
  achievements: async (id) =>
    id.startsWith('local:')
      ? { state: 'localGame' }
      : ({
          state: 'listed',
          unlocked: 3,
          total: 5,
          achievements: [
            { id: 'a1', name: 'First Steps', description: 'Finish the prologue.', unlocked: true, unlockedAt: now - 20 * day, icon: null, secret: false, rarity: 82.1 },
            { id: 'a2', name: 'Cartographer', description: 'Uncover every region.', unlocked: true, unlockedAt: now - 9 * day, icon: null, secret: false, rarity: 31.4 },
            { id: 'a3', name: 'Night Owl', description: 'Play after midnight.', unlocked: true, unlockedAt: now - 2 * day, icon: null, secret: false, rarity: 12.9 },
            { id: 'a4', name: 'Untouchable', description: 'Win a fight without damage.', unlocked: false, unlockedAt: null, icon: null, secret: false, rarity: 4.2 },
            { id: 'a5', name: 'Hidden', description: null, unlocked: false, unlockedAt: null, icon: null, secret: true, rarity: 1.1 },
          ],
        } as never),
  achievementIconUrl: () => '',
  launchOptions: async () => null,
  setLaunchOptions: async () => {},
  hiddenGames: async () => [],
  hideGame: async () => {},
  unhideGame: async () => {},
  localGame: async (id) => (id.startsWith('local:') ? ({ id, title: games.find((g) => g.id === id)!.title, executable: 'D:\\Games\\game.exe' } as never) : null),
  chooseExecutableFor: async () => ({ outcome: 'cancelled' }) as never,
  editLocalGame: async () => ({ outcome: 'noTitle' }) as never,
  removeLocalGame: async () => {},
  statistics: async () => ({
    games: games.length,
    playtimeMinutes: games.reduce((s, g) => s + (g.playtimeMinutes ?? 0), 0),
    achievementsUnlocked: 412,
    achievementsTotal: 980,
    achievementShare: 42,
    completeGames: games.filter((g) => g.progress === 100).length,
    gamesByStatus: { unplayed: 6, playing: 4, completed: 3, dropped: 1 },
    weeks: [180, 420, 95, 600, 310, 0, 520, 240].map((minutes, i) => ({ startsAt: now - (8 - i) * 7 * day, endsAt: now - (7 - i) * 7 * day, minutes })),
    mostPlayed: [...games].sort((a, b) => (b.playtimeMinutes ?? 0) - (a.playtimeMinutes ?? 0)).slice(0, 5),
  }),
  markFavorite: async (id, fav) => {
    games.find((g) => g.id === id)!.favorite = fav;
  },
  favoriteCount: async () => games.filter((g) => g.favorite).length,
  about: async () => ({ version: '0.1.0', license: 'MIT' }) as never,
  openAboutPage: async () => {},
  exportData: async () => ({ outcome: 'cancelled' }),
  coverHue: async (id) => ({ hue: hues.get(id) ?? 0, saturation: 0.55 }),
  install: async () => ({ outcome: 'started' }) as never,
  suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }) as never,
  findSteamTitles: async () => [],
  proposeCoverMatch: async () => null,
  coverMatch: async () => null,
  setCoverMatch: async () => {},
  importData: async () => ({ outcome: 'cancelled' }) as never,
} as Operations;

// ---------- Variant switcher ----------
function currentIndex() {
  const key = new URLSearchParams(location.search).get('variant') ?? VARIANTS[0].key;
  return Math.max(0, VARIANTS.findIndex((v) => v.key === key));
}

function Switcher() {
  const [index, setIndex] = useState(currentIndex);
  const [open, setOpen] = useState(true);
  const variant = VARIANTS[index];

  useEffect(() => {
    let style = document.getElementById('prototype-accent-style');
    if (!style) {
      style = document.createElement('style');
      style.id = 'prototype-accent-style';
      document.head.appendChild(style);
    }
    style.textContent = variant.css;
    const params = new URLSearchParams(location.search);
    params.set('variant', variant.key);
    history.replaceState(null, '', `?${params}`);
  }, [variant]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (t.closest('input, textarea, select, [contenteditable]')) return;
      if (e.key === 'ArrowLeft') setIndex((i) => (i - 1 + VARIANTS.length) % VARIANTS.length);
      if (e.key === 'ArrowRight') setIndex((i) => (i + 1) % VARIANTS.length);
    };
    addEventListener('keydown', onKey);
    return () => removeEventListener('keydown', onKey);
  }, []);

  const flipTheme = () => {
    const root = document.documentElement;
    root.dataset.theme = root.dataset.theme === 'light' ? 'dark' : 'light';
  };

  const pill: React.CSSProperties = { border: 0, background: '#111', color: '#ffd23f', width: 32, height: 32, borderRadius: 999, cursor: 'pointer', fontSize: 16 };
  return (
    <div style={{ position: 'fixed', left: '50%', bottom: 16, transform: 'translateX(-50%)', zIndex: 9999, font: '600 13px system-ui, sans-serif', color: '#111', display: 'grid', justifyItems: 'center', gap: 6 }}>
      {open && (
        <div style={{ background: '#fff8d6', border: '2px solid #ffd23f', borderRadius: 10, padding: '8px 12px', maxWidth: 520, fontWeight: 400, lineHeight: 1.4, boxShadow: '0 8px 24px rgba(0,0,0,.35)' }}>
          <b>Lime zeigt sich hier:</b> {variant.where}
        </div>
      )}
      <div style={{ display: 'flex', alignItems: 'center', gap: 4, background: '#ffd23f', borderRadius: 999, padding: 5, boxShadow: '0 8px 24px rgba(0,0,0,.4)' }}>
        <button style={pill} title="Vorherige Variante (←)" onClick={() => setIndex((i) => (i - 1 + VARIANTS.length) % VARIANTS.length)}>←</button>
        <span style={{ minWidth: 240, textAlign: 'center', padding: '0 8px' }}>
          PROTOTYPE · {variant.key} ({variant.name}) {index + 1}/{VARIANTS.length}
        </span>
        <button style={pill} title="Nächste Variante (→)" onClick={() => setIndex((i) => (i + 1) % VARIANTS.length)}>→</button>
        <button style={{ ...pill, width: 'auto', padding: '0 10px', fontSize: 12 }} onClick={flipTheme} title="Hell/Dunkel umschalten">Hell/Dunkel</button>
        <button style={{ ...pill, width: 'auto', padding: '0 10px', fontSize: 12 }} onClick={() => setOpen((o) => !o)}>{open ? 'Info aus' : 'Info'}</button>
      </div>
    </div>
  );
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App operations={ops} lang={systemLang()} />
  </StrictMode>,
);
createRoot(document.getElementById('prototype-switcher')!).render(<Switcher />);
