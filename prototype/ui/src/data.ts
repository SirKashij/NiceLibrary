// PROTOTYPE - invented placeholder data. No real games, no real artwork.

export type Source = 'steam' | 'local';
export type Status = 'unplayed' | 'playing' | 'completed' | 'dropped';

export interface Achievement {
  id: string;
  name: string;
  desc: string;
  unlocked: boolean;
  date?: string;
  rarity: number; // percent of players worldwide
  hidden: boolean;
}

export interface Session {
  date: string;
  minutes: number;
}

export interface Game {
  id: string;
  title: string;
  source: Source;
  installed: boolean;
  status: Status;
  minutes: number;
  lastPlayed: string | null;
  favorite: boolean;
  hue: number;
  achievements: Achievement[];
  sessions: Session[];
  collections: string[];
  manualPercent?: number;
}

export const TODAY = new Date('2026-11-25T12:00:00');

function rng(seed: number) {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function daysAgo(n: number) {
  const d = new Date(TODAY);
  d.setDate(d.getDate() - n);
  return d.toISOString();
}

const ACH = [
  ['First Steps', 'Finish the introduction.'],
  ['Collector', 'Find 50 hidden items.'],
  ['Night Owl', 'Play through a full night cycle.'],
  ['Untouchable', 'Clear a chapter without taking damage.'],
  ['Cartographer', 'Reveal the whole map.'],
  ['Old Friends', 'Reunite the crew.'],
  ['Speedrunner', 'Finish a chapter in under 10 minutes.'],
  ['Tinkerer', 'Craft 25 upgrades.'],
  ['Against the Tide', 'Win on the highest difficulty.'],
  ['Deep Pockets', 'Hold 10,000 coins at once.'],
  ['Storyteller', 'Read every journal entry.'],
  ['Last Light', 'See the true ending.'],
  ['Pacifist', 'Finish a chapter without a fight.'],
  ['Green Thumb', 'Grow every plant type.'],
  ['Perfect Run', 'Reach an S rank in every stage.'],
  ['Explorer', 'Visit every region.'],
  ['Quick Hands', 'Land a 50 hit combo.'],
  ['The Long Road', 'Travel 100 kilometres.'],
  ['Locksmith', 'Open 30 locked chests.'],
  ['Completionist', 'Unlock everything else.'],
];

function makeAchievements(seed: number, total: number, unlocked: number): Achievement[] {
  const r = rng(seed);
  return Array.from({ length: total }, (_, i) => {
    const [name, desc] = ACH[(i + seed) % ACH.length];
    const isUnlocked = i < unlocked;
    return {
      id: `a${i}`,
      name: total > ACH.length && i >= ACH.length ? `${name} II` : name,
      desc,
      unlocked: isUnlocked,
      date: isUnlocked ? daysAgo(Math.floor(r() * 300) + 1) : undefined,
      rarity: Math.round((isUnlocked ? 8 + r() * 80 : 0.5 + r() * 30) * 10) / 10,
      hidden: !isUnlocked && r() < 0.3,
    };
  });
}

function makeSessions(seed: number, last: number | null, count: number): Session[] {
  if (last === null) return [];
  const r = rng(seed * 7 + 3);
  let day = last;
  return Array.from({ length: count }, () => {
    const s = { date: daysAgo(day), minutes: 20 + Math.floor(r() * 170) };
    day += 1 + Math.floor(r() * 6);
    return s;
  });
}

let n = 0;
function g(
  title: string,
  source: Source,
  installed: boolean,
  status: Status,
  hours: number,
  last: number | null,
  favorite: boolean,
  hue: number,
  achTotal: number,
  achUnlocked: number,
  collections: string[] = [],
  manualPercent?: number,
): Game {
  n += 1;
  return {
    id: `g${n}`,
    title,
    source,
    installed,
    status,
    minutes: Math.round(hours * 60),
    lastPlayed: last === null ? null : daysAgo(last),
    favorite,
    hue,
    achievements: source === 'steam' ? makeAchievements(n * 13, achTotal, achUnlocked) : [],
    sessions: makeSessions(n, last, last === null ? 0 : Math.min(12, 2 + Math.floor(hours / 6))),
    collections,
    manualPercent,
  };
}

export const COLLECTIONS = ['Koop-Abende', 'Kurz & gut', 'Backlog 2026'];

export const GAMES: Game[] = [
  g('Aschenfeld', 'steam', true, 'playing', 46.5, 0, true, 18, 42, 29, ['Backlog 2026']),
  g('Neon Drift 2089', 'steam', true, 'playing', 12.2, 1, false, 300, 30, 9),
  g('Hollow Tide', 'steam', true, 'completed', 71, 9, true, 195, 48, 48),
  g('Kupferherz', 'local', true, 'playing', 8.4, 2, false, 28, 0, 0, ['Kurz & gut'], 35),
  g('Starloom', 'steam', false, 'completed', 33.7, 64, false, 255, 36, 31),
  g('Der Pilzkönig', 'steam', true, 'playing', 19.1, 4, true, 120, 25, 17, ['Koop-Abende']),
  g('Iron Meridian', 'steam', false, 'dropped', 5.3, 210, false, 215, 55, 6),
  g('Tiny Harbor', 'local', true, 'completed', 6.8, 30, false, 185, 0, 0, ['Kurz & gut'], 100),
  g('Echoes of Vahl', 'steam', true, 'playing', 88.9, 3, true, 275, 60, 51, ['Backlog 2026']),
  g('Rift Courier', 'steam', false, 'unplayed', 0, null, false, 340, 28, 0, ['Backlog 2026']),
  g('Mondgarten', 'steam', true, 'completed', 14.6, 21, false, 150, 20, 20, ['Kurz & gut']),
  g('Blackwater Rally', 'steam', true, 'playing', 27.3, 6, false, 5, 40, 22, ['Koop-Abende']),
  g('Sable & Thorn', 'local', true, 'unplayed', 0, null, false, 320, 0, 0),
  g('Orbit Janitor', 'steam', false, 'unplayed', 0.4, 400, false, 230, 18, 1),
  g('Papierdrachen', 'steam', true, 'playing', 3.9, 12, false, 45, 22, 5, ['Koop-Abende']),
  g('Glasswing', 'steam', false, 'completed', 9.2, 120, true, 170, 15, 14),
  g('Deep Signal', 'local', true, 'dropped', 2.1, 90, false, 205, 0, 0, [], 10),
  g('Frostline Express', 'steam', true, 'unplayed', 0, null, false, 200, 34, 0, ['Backlog 2026']),
];

export function percent(game: Game): number | null {
  if (game.achievements.length > 0) {
    const u = game.achievements.filter((a) => a.unlocked).length;
    return Math.round((u / game.achievements.length) * 100);
  }
  return game.manualPercent ?? null;
}

export function computeStats(games: Game[]) {
  const totalMinutes = games.reduce((s, x) => s + x.minutes, 0);
  const achTotal = games.reduce((s, x) => s + x.achievements.length, 0);
  const achUnlocked = games.reduce((s, x) => s + x.achievements.filter((a) => a.unlocked).length, 0);
  const statusCounts: Record<Status, number> = { unplayed: 0, playing: 0, completed: 0, dropped: 0 };
  games.forEach((x) => (statusCounts[x.status] += 1));
  const perfect = games.filter((x) => percent(x) === 100).length;
  const weeks = Array.from({ length: 8 }, (_, i) => ({ index: 7 - i, minutes: 0 }));
  games.forEach((x) =>
    x.sessions.forEach((s) => {
      const age = Math.floor((TODAY.getTime() - new Date(s.date).getTime()) / (7 * 86400000));
      if (age >= 0 && age < 8) weeks[7 - age].minutes += s.minutes;
    }),
  );
  const top = [...games].sort((a, b) => b.minutes - a.minutes).slice(0, 5);
  return { totalMinutes, achTotal, achUnlocked, statusCounts, perfect, weeks, top, count: games.length };
}
