// Smoke test of the built app: it starts, opens its window, and the interface
// reaches the application core through the real Tauri commands.
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { useApp } from './app';

describe('the built app', () => {
  const app = useApp();

  it('opens on the Library and shows the Installed Steam Games the core read', async () => {
    const t = await app.messages();

    expect(await app.text('main h1')).toBe(t['nav.library']);
    // The note appears only once `read_installed_games` answered with no
    // Games; a machine with Steam Games shows the list instead.
    const library = await app.text('main .note, main .games');
    const games = await app.execute<string[]>(
      'return Array.from(document.querySelectorAll("main .game-title")).map((h) => h.textContent)',
    );
    if (games.length === 0) expect(library).toBe(t['library.empty']);
    else expect(games.every((title) => title.trim() !== ''), games.join(', ')).toBe(true);
  });

  it('keeps its database in the user profile', () => {
    const appData = process.env.APPDATA;
    expect(appData, 'APPDATA is set on Windows').toBeDefined();
    expect(existsSync(join(appData!, 'io.github.sirkashij.nicelibrary', 'nicelibrary.db'))).toBe(true);
  });

  /** Loads `src` into an image in the app's window and answers whether it loaded. */
  async function loadImage(src: string): Promise<'loaded' | 'failed'> {
    await app.execute(
      `window.__imageCheck = 'loading';
       const image = new Image();
       image.onload = () => { window.__imageCheck = image.naturalWidth > 0 ? 'loaded' : 'failed'; };
       image.onerror = () => { window.__imageCheck = 'failed'; };
       image.src = arguments[0];`,
      src,
    );
    const deadline = Date.now() + 30_000;
    for (;;) {
      const state = await app.execute<string>('return window.__imageCheck');
      if (state !== 'loading') return state as 'loaded' | 'failed';
      if (Date.now() > deadline) throw new Error(`${src} neither loaded nor failed within 30 s`);
      await new Promise((r) => setTimeout(r, 250));
    }
  }

  it('loads a Steam Cover through the core’s cover URL scheme', async () => {
    const src = await app.execute<string>("return window.__TAURI_INTERNALS__.convertFileSrc('steam:620', 'cover')");

    expect(await loadImage(src)).toBe('loaded');
  });

  it('does not let the interface load images from the web itself', async () => {
    const steam = 'https://shared.steamstatic.com/store_item_assets/steam/apps/620/library_600x900.jpg';

    expect(await loadImage(steam)).toBe('failed');
  });

  it('hands a Launch to the core', async () => {
    // A Game that is in no Library, so nothing is opened: the core's own
    // refusal shows that the command and its argument reached it.
    const answer = await app.execute<string>(
      `return window.__TAURI_INTERNALS__.invoke('launch', { gameId: 'steam:0' }).then(
         (outcome) => 'launched: ' + JSON.stringify(outcome),
         (error) => String(error),
       )`,
    );

    expect(answer).toBe('no such game in the library: steam:0');
  });

  it('shows in the settings whether the core found Steam', async () => {
    const t = await app.messages();
    await app.execute(
      'Array.from(document.querySelectorAll("nav button")).find((b) => b.textContent === arguments[0]).click()',
      t['nav.settings'],
    );

    // The first line under the Steam heading only appears once the `steam_installation` command answered.
    // Only the Steam section: the key section below has a line from the start.
    const answer = await app.text('[aria-labelledby="settings-steam"] > :not(h2)');
    expect(answer === t['steam.notFound'] || answer.startsWith(t['steam.found']), answer).toBe(true);
  });

  it('shows in the settings whether a Steam Web API key is saved in the secret store', async () => {
    const t = await app.messages();
    await app.execute(
      'Array.from(document.querySelectorAll("nav button")).find((b) => b.textContent === arguments[0]).click()',
      t['nav.settings'],
    );

    // The row only appears once the `steam_web_api_key_stored` command answered from the Credential Manager.
    const answer = await app.text('[aria-labelledby="settings-key"] .settings-row > span');
    expect([t['key.stored'], t['key.notStored']]).toContain(answer);
  });
});
