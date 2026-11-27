import { useEffect, useState } from 'react';
import { useT, type MessageKey } from './i18n';
import type { Operations, Update, UpdateInstall } from './operations';

/** What installing an offered Update answered, for the user; `started` quits the app and needs no text. */
const INSTALL_PROBLEMS: Record<Exclude<UpdateInstall['outcome'], 'started'>, MessageKey> = {
  invalidSignature: 'updates.invalidSignature',
  notOffered: 'updates.notOffered',
};

/**
 * Asks the user whether to install an Update a check found. Nothing is
 * installed until they choose to; "Later" closes the offer and installs nothing.
 */
export function UpdateOffer({
  operations,
  update,
  onClose,
}: {
  operations: Operations;
  update: Update;
  onClose: () => void;
}) {
  const t = useT();
  const [installing, setInstalling] = useState(false);
  const [problem, setProblem] = useState<MessageKey>();

  const install = () => {
    setProblem(undefined);
    setInstalling(true);
    operations.installUpdate(update.version).then(
      (installed) => {
        // Started, the app quits; the window stays as it is until then.
        if (installed.outcome !== 'started') {
          setInstalling(false);
          setProblem(INSTALL_PROBLEMS[installed.outcome]);
        }
      },
      () => {
        setInstalling(false);
        setProblem('updates.installFailed');
      },
    );
  };

  return (
    <div className="tray-notice-backdrop">
      <div
        className="tray-notice"
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-offer-title"
        aria-describedby="update-offer-text"
      >
        <h2 id="update-offer-title">{t('updates.availableTitle')}</h2>
        <p id="update-offer-text" className="note">
          {t('updates.available', { version: update.version })}
        </p>
        {update.notes.trim() && <p className="muted update-notes">{update.notes}</p>}
        {installing && <p className="muted">{t('updates.installing')}</p>}
        {problem && (
          <p className="note" role="alert">
            {t(problem)}
          </p>
        )}
        <div className="field-row">
          <button type="button" className="button" autoFocus disabled={installing} onClick={install}>
            {t('updates.install')}
          </button>
          <button type="button" className="button" disabled={installing} onClick={onClose}>
            {t('updates.later')}
          </button>
        </div>
      </div>
    </div>
  );
}

/** What checking by hand last found, shown below the button. */
type Checked = { state: 'checking' } | { state: 'upToDate' } | { state: 'failed' };

/**
 * The Update settings: whether the app checks for Updates on its own, and
 * checking by hand, which works with the automatic check off too. An Update
 * found goes to `onFound`, which offers it.
 */
export function UpdateSettings({ operations, onFound }: { operations: Operations; onFound: (update: Update) => void }) {
  const t = useT();
  // `undefined` while loading; what is saved once the core answered.
  const [automatic, setAutomatic] = useState<boolean>();
  const [problem, setProblem] = useState<MessageKey>();
  const [checked, setChecked] = useState<Checked>();

  useEffect(() => {
    let active = true;
    operations.checksForUpdates().then(
      (on) => active && setAutomatic(on),
      () => active && setProblem('updates.loadFailed'),
    );
    return () => {
      active = false;
    };
  }, [operations]);

  const change = (on: boolean) => {
    setProblem(undefined);
    operations.setCheckForUpdates(on).then(
      () => setAutomatic(on),
      () => setProblem('updates.saveFailed'),
    );
  };

  const check = () => {
    setChecked({ state: 'checking' });
    operations.checkForUpdate().then(
      (found) => {
        if (found.outcome === 'available') {
          setChecked(undefined);
          onFound(found.update);
        } else {
          setChecked({ state: 'upToDate' });
        }
      },
      () => setChecked({ state: 'failed' }),
    );
  };

  return (
    <section className="settings-section" aria-labelledby="settings-updates">
      <h2 id="settings-updates">{t('settings.updates')}</h2>
      {automatic !== undefined && (
        <>
          <label className="checkbox-row">
            <input
              type="checkbox"
              aria-describedby="check-for-updates-hint"
              checked={automatic}
              onChange={(event) => change(event.target.checked)}
            />
            {t('updates.automatic')}
          </label>
          <p id="check-for-updates-hint" className="muted">
            {t(automatic ? 'updates.automaticOn' : 'updates.automaticOff')}
          </p>
        </>
      )}
      {problem && (
        <p className="note" role="alert">
          {t(problem)}
        </p>
      )}
      <button type="button" className="button" disabled={checked?.state === 'checking'} onClick={check}>
        {t('updates.check')}
      </button>
      {checked?.state === 'checking' && <p className="muted">{t('updates.checking')}</p>}
      {checked?.state === 'upToDate' && <p className="muted">{t('updates.upToDate')}</p>}
      {checked?.state === 'failed' && (
        <p className="note" role="alert">
          {t('updates.checkFailed')}
        </p>
      )}
    </section>
  );
}
