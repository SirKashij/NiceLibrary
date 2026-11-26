// Drives the built app through WebDriver. `useApp()` starts tauri-driver,
// launches the app in a fresh session before the tests of a file and closes
// both afterwards. Tests only see the handle it returns.
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterAll, beforeAll } from 'vitest';
import { messages, type Lang, type MessageKey } from '../src/i18n';

const root = resolve(import.meta.dirname, '..');
// Keep in step with the version installed in .github/workflows/ci.yml.
const TAURI_DRIVER_INSTALL = 'cargo install tauri-driver --version 2.1.0 --locked';
const DRIVER_PORT = 4444;
const DRIVER_URL = `http://127.0.0.1:${DRIVER_PORT}`;
const WAIT_MS = 10_000;
// The W3C name under which WebDriver returns an element reference.
const ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';

export interface App {
  /** Text of the first element matching the CSS selector, waiting until it appears. */
  text(selector: string): Promise<string>;
  /**
   * Like `text`, but only inside the region labelled `labelId`, the element
   * with `aria-labelledby="<labelId>"`. `:scope` in the selector is the region.
   */
  textIn(labelId: string, selector: string): Promise<string>;
  /** The interface texts in the language the app chose. */
  messages(): Promise<Record<MessageKey, string>>;
  /** Runs a script in the app's window and returns its result. */
  execute<T>(script: string, ...args: unknown[]): Promise<T>;
}

/** A WebDriver command the driver answered with an error. */
class WebDriverError extends Error {
  constructor(
    message: string,
    /** The W3C error code, such as "no such element". */
    readonly code: string,
  ) {
    super(message);
  }
}

/** Whether a WebDriver error means the element is not, or no longer, in the page. */
function notInPage(error: unknown): boolean {
  return error instanceof WebDriverError && (error.code === 'no such element' || error.code === 'stale element reference');
}

/** Registers hooks that launch the app for the tests of the calling file. */
export function useApp(): App {
  let driver: ChildProcess | undefined;
  let session: string | undefined;

  beforeAll(async () => {
    const application = appPath();
    if (await driverAnswers()) {
      throw new Error(`Port ${DRIVER_PORT} is in use, probably by a tauri-driver left over from an earlier run. Stop it first.`);
    }
    driver = startDriver();
    await waitForDriver(driver);
    const created = await command<{ sessionId: string }>('POST', '/session', {
      capabilities: { alwaysMatch: { 'tauri:options': { application } } },
    });
    session = created.sessionId;
  });

  afterAll(async () => {
    if (session) await command('DELETE', `/session/${session}`).catch(() => {});
    if (driver) stopProcessTree(driver);
  });

  const inSession = <T>(method: string, path: string, body?: unknown) => {
    if (!session) throw new Error('The app did not start; see the error of the beforeAll hook.');
    return command<T>(method, `/session/${session}${path}`, body);
  };

  const execute = <T>(script: string, ...args: unknown[]) => inSession<T>('POST', '/execute/sync', { script, args });

  /** The first element matching `selector`, searched from the page or, with `from`, inside that element. */
  const find = async (selector: string, from?: string) => {
    const path = from === undefined ? '/element' : `/element/${from}/element`;
    const element = await inSession<Record<string, string>>('POST', path, { using: 'css selector', value: selector });
    return element[ELEMENT_KEY];
  };

  const textOf = (element: string) => inSession<string>('GET', `/element/${element}/text`);

  return {
    execute,

    text(selector) {
      return poll(async () => textOf(await find(selector)), notInPage);
    },

    textIn(labelId, selector) {
      // Region, element and text are read in one attempt, so a region the
      // interface rendered anew in between is simply found again.
      return poll(async () => textOf(await find(selector, await find(`[aria-labelledby="${labelId}"]`))), notInPage);
    },

    async messages() {
      const lang = await execute<string>('return document.documentElement.lang');
      return messages[lang as Lang] ?? messages.en;
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

/** Stops tauri-driver together with the msedgedriver and app it started. */
function stopProcessTree(driver: ChildProcess): void {
  if (driver.exitCode !== null || driver.pid === undefined) return;
  if (process.platform === 'win32') {
    try {
      execFileSync('taskkill', ['/PID', String(driver.pid), '/T', '/F'], { stdio: 'ignore' });
    } catch {
      // Already gone.
    }
  } else {
    driver.kill();
  }
}

async function driverAnswers(): Promise<boolean> {
  try {
    await command('GET', '/status');
    return true;
  } catch {
    return false;
  }
}

async function waitForDriver(driver: ChildProcess): Promise<void> {
  let failure: Error | undefined;
  driver.once('error', (error) => {
    failure = new Error(`tauri-driver could not be started (${error.message}). Install it: ${TAURI_DRIVER_INSTALL}`);
  });
  driver.once('exit', (code) => {
    failure ??= new Error(`tauri-driver exited with code ${code}.`);
  });
  await poll(
    async () => {
      if (failure) throw failure;
      await command('GET', '/status');
    },
    () => !failure,
  );
}

async function command<T = unknown>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await fetch(DRIVER_URL + path, {
    method,
    headers: body === undefined ? undefined : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const raw = await response.text();
  let payload: { value?: T & { error?: string; message?: string } } | undefined;
  try {
    payload = JSON.parse(raw);
  } catch {
    // Not a WebDriver answer; reported below with the raw text.
  }
  if (!response.ok || payload === undefined) {
    const error = payload?.value?.error ?? `HTTP ${response.status}`;
    const message = payload?.value?.message ?? raw;
    throw new WebDriverError(`WebDriver ${method} ${path} failed: ${error}: ${message}`, error);
  }
  return payload.value as T;
}

/** Retries `attempt` while `retryable` accepts its error, until the wait time is over. */
async function poll<T>(attempt: () => Promise<T>, retryable: (error: unknown) => boolean): Promise<T> {
  const deadline = Date.now() + WAIT_MS;
  for (;;) {
    try {
      return await attempt();
    } catch (error) {
      if (!retryable(error) || Date.now() > deadline) throw error;
      await new Promise((r) => setTimeout(r, 200));
    }
  }
}
