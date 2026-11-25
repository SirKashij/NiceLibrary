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

  it('shows in the settings whether the core found Steam', async () => {
    const t = await app.messages();
    await app.execute(
      'Array.from(document.querySelectorAll("nav button")).find((b) => b.textContent === arguments[0]).click()',
      t['nav.settings'],
    );

    // The first line under the Steam heading only appears once the `steam_installation` command answered.
    const answer = await app.text('.settings-section > :not(h2)');
    expect(answer === t['steam.notFound'] || answer.startsWith(t['steam.found']), answer).toBe(true);
  });
});
