// Decides whether a shell command stops processes by name instead of by PID.
// Stopping by name also hits other programs that share the name: killing every
// msedgewebview2.exe once crashed an unrelated WebView2 app on a developer
// machine. Used by the PreToolUse hook in .claude/settings.json.

const STOP = String.raw`(?:Stop-Process|spps|kill)`;

const rules: RegExp[] = [
  // Stop-Process -Name x, kill -ProcessName x
  new RegExp(String.raw`\b${STOP}\b[^|]*\s-(?:Name|ProcessName)\b`, 'i'),
  // taskkill /IM x.exe
  /\btaskkill\b[^|]*\s[/-]im\b/i,
  /\b(?:pkill|killall)\b/i,
  // wmic process where name=... delete | call terminate
  /\bwmic\b.*\bprocess\b.*\b(?:delete|terminate)\b/i,
];

/** True if `command` stops processes selected by name. */
export function killsByName(command: string): boolean {
  return command.split(/;|&&|\|\||\r?\n/).some((statement) => rules.some((rule) => rule.test(statement)) || pipesNameToKill(statement));
}

/** Get-Process <name> | Stop-Process, ps ... | xargs kill */
function pipesNameToKill(statement: string): boolean {
  const stages = statement.split('|').map((stage) => stage.trim());
  const source = stages.findIndex((stage) => /^(?:Get-Process|gps|ps|pgrep)\b/i.test(stage) && !/\s-Id\b/i.test(stage));
  return source >= 0 && stages.slice(source + 1).some((stage) => new RegExp(String.raw`^(?:${STOP}|xargs\s+kill)\b`, 'i').test(stage));
}
