// Drives the built app through WebDriver. `useApp()` starts tauri-driver,
// launches the app in a fresh session before the tests of a file and closes
// both afterwards. Tests only see the handle it returns.
import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterAll, beforeAll } from 'vitest';
import { de } from '../src/i18n/de';
import { en, type MessageKey } from '../src/i18n/en';

const root = resolve(import.meta.dirname, '..');
const DRIVER_PORT = 4444;
const DRIVER_URL = `http://127.0.0.1:${DRIVER_PORT}`;
const WAIT_MS = 10_000;
// The W3C name under which WebDriver returns an element reference.
const ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';

export interface App {
  /** Text of the first element matching the CSS selector, waiting until it appears. */
  text(selector: string): Promise<string>;
  /** The interface texts in the language the app chose. */
  messages(): Promise<Record<MessageKey, string>>;
  /** Runs a script in the app's window and returns its result. */
  execute<T>(script: string, ...args: unknown[]): Promise<T>;
}

/** Registers hooks that launch the app for the tests of the calling file. */
export function useApp(): App {
  let driver: ChildProcess | undefined;
  let session: string | undefined;

  beforeAll(async () => {
    const application = appPath();
    driver = startDriver();
    await waitForDriver(driver);
    const created = await command<{ sessionId: string }>('POST', '/session', {
      capabilities: { alwaysMatch: { 'tauri:options': { application } } },
    });
    session = created.sessionId;
  });

  afterAll(async () => {
    if (session) await command('DELETE', `/session/${session}`).catch(() => {});
    driver?.kill();
  });

  const inSession = <T>(method: string, path: string, body?: unknown) => {
    if (!session) throw new Error('The app did not start; see the error of the beforeAll hook.');
    return command<T>(method, `/session/${session}${path}`, body);
  };

  const execute = <T>(script: string, ...args: unknown[]) => inSession<T>('POST', '/execute/sync', { script, args });

  return {
    execute,

    async text(selector) {
      const element = await poll(() =>
        inSession<Record<string, string>>('POST', '/element', { using: 'css selector', value: selector }),
      );
      return inSession<string>('GET', `/element/${element[ELEMENT_KEY]}/text`);
    },

    async messages() {
      const lang = await execute<string>('return document.documentElement.lang');
      return lang === 'de' ? de : en;
    },
  };
}

/**
 * The app binary under test: $NICELIBRARY_APP, or else the most recently
 * built of the release and debug binaries.
 */
function appPath(): string {
  const fromEnv = process.env.NICELIBRARY_APP;
  if (fromEnv) {
    if (!existsSync(fromEnv)) throw new Error(`NICELIBRARY_APP points to ${fromEnv}, which does not exist.`);
    return resolve(fromEnv);
  }
  const exe = process.platform === 'win32' ? 'nicelibrary.exe' : 'nicelibrary';
  const built = ['release', 'debug']
    .map((profile) => resolve(root, 'target', profile, exe))
    .filter((path) => existsSync(path))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  if (built.length === 0) {
    throw new Error('No built app found. Build it first: npm run tauri -- build --debug --no-bundle');
  }
  return built[0];
}

function startDriver(): ChildProcess {
  const args = ['--port', String(DRIVER_PORT)];
  const edgeDriver = resolve(root, 'target', 'e2e', 'msedgedriver.exe');
  if (existsSync(edgeDriver)) args.push('--native-driver', edgeDriver);
  return spawn('tauri-driver', args, { stdio: ['ignore', 'inherit', 'inherit'] });
}

async function waitForDriver(driver: ChildProcess): Promise<void> {
  let failure: Error | undefined;
  driver.once('error', (error) => {
    failure = new Error(`tauri-driver could not be started (${error.message}). Install it: cargo install tauri-driver --locked`);
  });
  driver.once('exit', (code) => {
    failure ??= new Error(`tauri-driver exited with code ${code}.`);
  });
  await poll(async () => {
    if (failure) throw Object.assign(failure, { final: true });
    await command('GET', '/status');
  });
}

async function command<T = unknown>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await fetch(DRIVER_URL + path, {
    method,
    headers: body === undefined ? undefined : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const payload = (await response.json()) as { value: T & { error?: string; message?: string } };
  if (!response.ok) {
    throw new Error(`WebDriver ${method} ${path} failed: ${payload.value.error}: ${payload.value.message}`);
  }
  return payload.value;
}

/** Retries `attempt` until it succeeds or the wait time is over. */
async function poll<T>(attempt: () => Promise<T>): Promise<T> {
  const deadline = Date.now() + WAIT_MS;
  for (;;) {
    try {
      return await attempt();
    } catch (error) {
      if ((error as { final?: boolean }).final || Date.now() > deadline) throw error;
      await new Promise((r) => setTimeout(r, 200));
    }
  }
}
