import { useEffect, useState } from 'react';
import { useT, type MessageKey } from './i18n';
import type { LimitedMode, Operations } from './operations';

type Reason = Exclude<LimitedMode['outcome'], 'off'>;

/** Why the app runs in Limited Mode, for the user. */
const REASONS: Record<Reason, MessageKey> = {
  noKey: 'limited.noKey',
  keyRejected: 'limited.keyRejected',
  gameDetailsPrivate: 'limited.gameDetailsPrivate',
  steamUnreachable: 'limited.steamUnreachable',
};

/**
 * A quiet hint above the Library while the app runs in Limited Mode: why,
 * and the action that fixes it. Asks the core again each time `syncsEnded`
 * changes, so it disappears once a Sync succeeds.
 */
export function LimitedModeHint({
  operations,
  syncsEnded,
  onOpenKeySettings,
  onSync,
}: {
  operations: Operations;
  /** How many Syncs ended while the app runs, successful or not. */
  syncsEnded: number;
  /** Opens the settings section of the Steam Web API key. */
  onOpenKeySettings: () => void;
  /** Starts a Sync, as Sync now in the sidebar does. */
  onSync: () => void;
}) {
  const t = useT();
  const [mode, setMode] = useState<LimitedMode>();
  // Whether a Sync asked for here has not ended yet.
  const [syncing, setSyncing] = useState(false);
  const [problem, setProblem] = useState<MessageKey>();

  useEffect(() => {
    let active = true;
    setSyncing(false);
    // Without an answer no hint is shown: Limited Mode is not certain.
    operations.limitedMode().then(
      (answer) => active && setMode(answer),
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations, syncsEnded]);

  if (!mode || mode.outcome === 'off') return null;

  const sync = () => {
    setProblem(undefined);
    setSyncing(true);
    onSync();
  };
  const openPrivacySettings = () => {
    setProblem(undefined);
    operations.openSteamPrivacySettings().catch(() => setProblem('limited.privacyFailed'));
  };
  const syncButton = (label: MessageKey) => (
    <button type="button" className="button" disabled={syncing} onClick={sync}>
      {t(syncing ? 'sync.syncing' : label)}
    </button>
  );

  return (
    <section className="limited-mode" aria-labelledby="limited-mode-title">
      <h2 id="limited-mode-title">{t('limited.title')}</h2>
      <p className="note">{t(REASONS[mode.outcome])}</p>
      <div className="settings-actions">
        {mode.outcome === 'noKey' && (
          <button type="button" className="button" onClick={onOpenKeySettings}>
            {t('limited.setUp')}
          </button>
        )}
        {mode.outcome === 'keyRejected' && (
          <button type="button" className="button" onClick={onOpenKeySettings}>
            {t('limited.openSettings')}
          </button>
        )}
        {mode.outcome === 'gameDetailsPrivate' && (
          <>
            <button type="button" className="button" onClick={openPrivacySettings}>
              {t('limited.openPrivacySettings')}
            </button>
            {syncButton('limited.syncAgain')}
          </>
        )}
        {mode.outcome === 'steamUnreachable' && syncButton('limited.retry')}
      </div>
      {problem && (
        <p className="note" role="alert">
          {t(problem)}
        </p>
      )}
    </section>
  );
}
