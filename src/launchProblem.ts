import type { MessageKey } from './i18n';
import type { LaunchOutcome } from './operations';

const PROBLEMS: Record<LaunchOutcome['outcome'], MessageKey | undefined> = {
  launched: undefined,
  steamNotFound: 'launch.steamNotFound',
  executableNotFound: 'launch.executableNotFound',
  workingDirectoryNotFound: 'launch.workingDirectoryNotFound',
};

/** Why a Launch did not start its Game, as a message with a `{title}`; `undefined` when it did. */
export function whyNotLaunched({ outcome }: LaunchOutcome): MessageKey | undefined {
  return PROBLEMS[outcome];
}
