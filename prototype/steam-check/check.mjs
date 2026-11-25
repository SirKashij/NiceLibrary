// PROTOTYPE - throwaway. Question: which Steam data can the app really get?
//   1. Does the Web API return the key owner's own games/achievements (also with a private profile)?
//   2. Is the Steam app list (for matching local games to covers) retrievable?
//   3. What do the local Steam files offer as a fallback?
// Run: node prototype/steam-check/check.mjs   (needs env STEAM_API_KEY; prints no key and no SteamID)
import { execSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const key = process.env.STEAM_API_KEY;
if (!key) { console.error('STEAM_API_KEY is not set'); process.exit(1); }

function steamPath() {
  if (process.env.STEAM_PATH) return process.env.STEAM_PATH;
  const out = execSync('reg query "HKCU\\Software\\Valve\\Steam" /v SteamPath', { encoding: 'utf8' });
  return out.match(/SteamPath\s+REG_SZ\s+(.+)/)[1].trim();
}

async function api(path, params) {
  const url = new URL(`https://api.steampowered.com/${path}`);
  url.searchParams.set('key', key);
  for (const [k, v] of Object.entries(params)) url.searchParams.set(k, String(v));
  const res = await fetch(url);
  let body = null;
  try { body = await res.json(); } catch { /* not JSON */ }
  return { status: res.status, body };
}

const line = (label, value) => console.log(`${label.padEnd(46)} ${value}`);

// --- local files ---
const root = steamPath();
const login = readFileSync(join(root, 'config', 'loginusers.vdf'), 'utf8');
const users = [...login.matchAll(/"(7656\d{13})"\s*\{([^}]*)\}/g)].map((m) => ({ id: m[1], recent: /"MostRecent"\s+"1"/.test(m[2]) }));
const steamId = (users.find((u) => u.recent) ?? users[0])?.id;
console.log('== Local files');
line('Steam installation found', existsSync(root));
line('Accounts in loginusers.vdf', users.length);
line('SteamID detected', Boolean(steamId));

const libs = readFileSync(join(root, 'steamapps', 'libraryfolders.vdf'), 'utf8');
const libPaths = [...libs.matchAll(/"path"\s+"([^"]+)"/g)].map((m) => m[1].replace(/\\\\/g, '\\'));
let manifests = 0;
for (const p of libPaths) {
  const dir = join(p, 'steamapps');
  if (existsSync(dir)) manifests += readdirSync(dir).filter((f) => /^appmanifest_\d+\.acf$/.test(f)).length;
}
line('Library folders', libPaths.length);
line('Installed games (appmanifest files)', manifests);

const accountId = (BigInt(steamId) - 76561197960265728n).toString();
const localConfig = join(root, 'userdata', accountId, 'config', 'localconfig.vdf');
if (existsSync(localConfig)) {
  const text = readFileSync(localConfig, 'utf8');
  line('localconfig.vdf: apps with "Playtime"', (text.match(/"Playtime"\s+"\d+"/g) ?? []).length);
  line('localconfig.vdf: apps with "LastPlayed"', (text.match(/"LastPlayed"\s+"\d+"/g) ?? []).length);
} else {
  line('localconfig.vdf', 'not found');
}

// --- Web API ---
let lastGames = [];
for (const [i, u] of users.entries()) {
const steamId = u.id; // shadows the most-recent id: every account on this PC is tested
console.log(`\n== Web API, account ${i + 1} of ${users.length}${u.recent ? ' (most recent login)' : ''}`);
const summary = await api('ISteamUser/GetPlayerSummaries/v2/', { steamids: steamId });
const vis = summary.body?.response?.players?.[0]?.communityvisibilitystate;
line('GetPlayerSummaries HTTP', summary.status);
line('Profile visibility (3 = public, 1 = private)', vis ?? 'unknown');

const owned = await api('IPlayerService/GetOwnedGames/v1/', { steamid: steamId, include_appinfo: 1, include_played_free_games: 1 });
const games = owned.body?.response?.games ?? [];
line('GetOwnedGames HTTP', owned.status);
line('Owned games returned', owned.body?.response?.game_count ?? 0);
line('Games with playtime > 0', games.filter((g) => g.playtime_forever > 0).length);
line('Fields per game', games[0] ? Object.keys(games[0]).join(', ') : '-');

const recent = await api('IPlayerService/GetRecentlyPlayedGames/v1/', { steamid: steamId });
line('GetRecentlyPlayedGames HTTP / count', `${recent.status} / ${recent.body?.response?.total_count ?? 0}`);

const candidates = [...games].sort((a, b) => b.playtime_forever - a.playtime_forever).slice(0, 8);
let tested = false;
for (const g of candidates) {
  const ach = await api('ISteamUserStats/GetPlayerAchievements/v1/', { steamid: steamId, appid: g.appid, l: 'german' });
  const list = ach.body?.playerstats?.achievements;
  if (!list) { line(`GetPlayerAchievements (top game) HTTP`, `${ach.status} ${ach.body?.playerstats?.error ?? ''}`); continue; }
  line('GetPlayerAchievements HTTP', ach.status);
  line('  achievements total / unlocked', `${list.length} / ${list.filter((a) => a.achieved).length}`);
  line('  fields', Object.keys(list[0]).join(', '));
  const schema = await api('ISteamUserStats/GetSchemaForGame/v2/', { appid: g.appid, l: 'german' });
  const sch = schema.body?.game?.availableGameStats?.achievements ?? [];
  line('GetSchemaForGame HTTP / achievements', `${schema.status} / ${sch.length}`);
  line('  schema fields', sch[0] ? Object.keys(sch[0]).join(', ') : '-');
  line('  hidden achievements in schema', sch.filter((a) => a.hidden).length);
  const global = await api('ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/', { gameid: g.appid });
  line('GetGlobalAchievementPercentages HTTP / count', `${global.status} / ${global.body?.achievementpercentages?.achievements?.length ?? 0}`);
  tested = true;
  break;
}
if (!tested) line('Achievements', 'no game with readable achievements among the top 8');
if (games.length) lastGames = games;
}

console.log('\n== App list (cover matching for local games)');
const store = await api('IStoreService/GetAppList/v1/', { max_results: 5, include_games: 1 });
const apps = store.body?.response?.apps ?? [];
line('IStoreService/GetAppList HTTP / apps', `${store.status} / ${apps.length}`);
line('  fields', apps[0] ? Object.keys(apps[0]).join(', ') : '-');
line('  paging (have_more_results)', store.body?.response?.have_more_results ?? '-');
const old = await api('ISteamApps/GetAppList/v2/', {});
line('ISteamApps/GetAppList (old) HTTP / apps', `${old.status} / ${old.body?.applist?.apps?.length ?? 0}`);

if (lastGames[0]) {
  const id = lastGames[0].appid;
  for (const [name, url] of [
    ['library_600x900.jpg', `https://steamcdn-a.akamaihd.net/steam/apps/${id}/library_600x900.jpg`],
    ['header.jpg', `https://cdn.cloudflare.steamstatic.com/steam/apps/${id}/header.jpg`],
  ]) {
    const r = await fetch(url, { method: 'HEAD' });
    line(`Cover ${name} HTTP`, r.status);
  }
}
