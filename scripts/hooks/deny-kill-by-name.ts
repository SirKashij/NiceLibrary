// Claude Code PreToolUse hook (.claude/settings.json): refuses Bash and
// PowerShell commands that stop processes by name. Reads the hook payload
// from stdin and answers with a deny decision, or nothing to let it pass.
import { readFileSync } from 'node:fs';
import { killsByName } from './kill-by-name.ts';

const payload = JSON.parse(readFileSync(0, 'utf8')) as { tool_input?: { command?: unknown } };
const command = payload.tool_input?.command;

if (typeof command === 'string' && killsByName(command)) {
  console.log(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        permissionDecision: 'deny',
        permissionDecisionReason:
          'Stopping processes by name also hits other programs on this machine (e.g. every WebView2 app). Stop the process tree you started instead: taskkill /PID <pid> /T /F, or Stop-Process -Id <pid>.',
      },
    }),
  );
}
