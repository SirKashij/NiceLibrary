// Smoke test of the built app: it starts, opens its window, and the interface
// reaches the application core through the real Tauri commands.
import { describe, expect, it } from 'vitest';
import { useApp } from './app';

describe('the built app', () => {
  const app = useApp();

  it('opens on the Library and shows the empty Library from the core', async () => {
    const t = await app.messages();

    expect(await app.text('main h1')).toBe(t['nav.library']);
    // The empty-Library note only appears once the `library` command answered.
    expect(await app.text('main .note')).toBe(t['library.empty']);
  });
});
