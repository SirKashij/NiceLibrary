// The app in the Tray: closing the window keeps the app running, a second
// start brings the running app forward, and the Tray menu quits it. The
// tests run in order on one app; each builds on the one before.
import { spawn } from 'node:child_process';
import { describe, expect, it } from 'vitest';
import { appPath, useApp } from './app';
import { chooseInTrayMenu, close, mainWindow, running } from './windows';

/** Waits until `condition` holds, failing with `what` after `ms`. */
async function until(condition: () => boolean | Promise<boolean>, what: string, ms = 10_000): Promise<void> {
  const deadline = Date.now() + ms;
  while (!(await condition())) {
    if (Date.now() > deadline) throw new Error(`Gave up waiting: ${what}`);
    await new Promise((r) => setTimeout(r, 200));
  }
}

describe('the app in the Tray', () => {
  const app = useApp();

  it('keeps running in the Tray when its window is closed', async (context) => {
    const pid = app.processId();
    await app.messages();
    // The user's own setting decides; with close-to-tray off there is no Tray to test.
    const closeToTray = await app.execute<boolean>("return window.__TAURI_INTERNALS__.invoke('close_to_tray')");
    if (!closeToTray) context.skip();
    const window = mainWindow(pid);
    expect(window.visible).toBe(true);

    close(window);

    // The very first close of this profile shows the Tray notice instead; reading it closes the window.
    await until(async () => {
      const notice = await app.execute<boolean>(
        `const ok = document.querySelector('.tray-notice button');
         if (ok) ok.click();
         return ok !== null;`,
      );
      return !notice && !mainWindow(pid).visible;
    }, 'the window to hide');
    expect(running(pid)).toBe(true);
    // The interface goes on answering while the window is hidden.
    expect(await app.execute<number>('return 1 + 1')).toBe(2);
  });

  it('brings the running app forward when it is started a second time', async () => {
    const pid = app.processId();

    const second = spawn(appPath(), [], { stdio: 'ignore' });
    const exitCode = await new Promise<number | 'still running'>((done) => {
      const timer = setTimeout(() => {
        // Stops only the process this test started, never others by name.
        second.kill();
        done('still running');
      }, 15_000);
      second.once('exit', (code) => {
        clearTimeout(timer);
        done(code ?? 'still running');
      });
    });

    expect(exitCode, 'the second start hands over to the running app and ends').toBe(0);
    await until(() => mainWindow(pid).visible, 'the window of the running app to show');
  });

  it('quits from the Tray menu', async () => {
    const t = await app.messages();
    const pid = app.processId();

    chooseInTrayMenu(pid, t['tray.quit']);

    await until(() => !running(pid), 'the app to quit');
  });
});
