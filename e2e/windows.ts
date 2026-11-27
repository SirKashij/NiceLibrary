// What the user and Windows do to the app's windows and Tray icon, for the
// end-to-end tests: a click on the close button, a right click on the Tray
// icon, a click in its menu. WebDriver only reaches the page inside the
// window, so these go through Win32 window messages (windows.ps1).
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';

const SCRIPT = resolve(import.meta.dirname, 'windows.ps1');

const WM_CLOSE = 0x0010;
const WM_RBUTTONUP = 0x0205;
/**
 * The message the Tray icon asks the shell to send it on a click (its
 * callback message). It is the tray-icon crate's choice (`WM_USER_TRAYICON`
 * in its Windows implementation); keep it in step when that crate changes.
 */
const WM_USER_TRAYICON = 6002;
/** The window class of the tray-icon crate's hidden window, which owns the Tray icon and its menu. */
const TRAY_WINDOW_CLASS = 'tray_icon_app';

export interface Window {
  hwnd: number;
  className: string;
  title: string;
  visible: boolean;
}

function run(args: string[]): string {
  return execFileSync('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', SCRIPT, ...args], {
    encoding: 'utf8',
  }).trim();
}

/** The process id of the app that tauri-driver (`driverPid`) started. */
export function appProcess(driverPid: number): number {
  return Number(run(['-Action', 'app', '-ProcessId', String(driverPid)]));
}

/** The top-level windows of the process `pid`, visible or not. */
export function windowsOf(pid: number): Window[] {
  return JSON.parse(run(['-Action', 'windows', '-ProcessId', String(pid)])) as Window[];
}

/** The app's main window: its titled top-level window, found by the title tauri.conf.json gives it. */
export function mainWindow(pid: number, title = 'NiceLibrary'): Window {
  const window = windowsOf(pid).find((w) => w.title === title && w.className !== TRAY_WINDOW_CLASS);
  if (!window) throw new Error(`The app (process ${pid}) has no window titled ${title}.`);
  return window;
}

function post(hwnd: number, message: number, wparam = 0, lparam = 0): void {
  run(['-Action', 'post', '-Window', String(hwnd), '-Message', String(message), '-WParam', String(wparam), '-LParam', String(lparam)]);
}

/** Closes the window as its close button does. */
export function close(window: Window): void {
  post(window.hwnd, WM_CLOSE);
}

/** Right-clicks the app's Tray icon, as the shell reports it, and chooses `item` in the menu that opens. */
export function chooseInTrayMenu(pid: number, item: string): void {
  const tray = windowsOf(pid).find((w) => w.className === TRAY_WINDOW_CLASS);
  if (!tray) throw new Error(`The app (process ${pid}) has no Tray icon.`);
  post(tray.hwnd, WM_USER_TRAYICON, 0, WM_RBUTTONUP);
  run(['-Action', 'menu', '-ProcessId', String(pid), '-Item', item]);
}

/** Whether the process `pid` still runs. */
export function running(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}
