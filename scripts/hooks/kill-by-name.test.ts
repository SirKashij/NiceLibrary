import { describe, expect, it } from 'vitest';
import { killsByName } from './kill-by-name.ts';

describe('killsByName', () => {
  it.each([
    'Get-Process msedgewebview2 | Stop-Process -Force',
    'Get-Process nicelibrary, msedgewebview2 -ErrorAction SilentlyContinue | Stop-Process -Force',
    'Stop-Process -Name msedgewebview2',
    'stop-process -processname node -Force',
    'kill -Name node',
    'spps -Name node',
    'Get-Process node | kill',
    'taskkill /IM msedgewebview2.exe /F',
    'taskkill /f /im node.exe',
    'taskkill -im node.exe',
    'pkill -f nicelibrary',
    'killall node',
    'wmic process where name="node.exe" delete',
    'wmic process where "name=\'node.exe\'" call terminate',
    'cd x && Get-Process node | Stop-Process',
  ])('blocks %s', (command) => {
    expect(killsByName(command)).toBe(true);
  });

  it.each([
    'Stop-Process -Id 1234 -Force',
    'Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue',
    'Get-Process -Id 1234 | Stop-Process',
    'Get-CimInstance Win32_Process | Where-Object CommandLine -Match nicelibrary | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }',
    'taskkill /PID 1234 /T /F',
    'kill 1234',
    'kill -9 1234',
    'Get-Process msedgewebview2',
    'ps aux',
    'npm run test:e2e',
  ])('allows %s', (command) => {
    expect(killsByName(command)).toBe(false);
  });
});
