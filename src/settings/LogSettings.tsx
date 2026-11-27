import { useState } from 'react';
import { useT } from '../i18n';
import type { Operations } from '../operations';

/** The log file for bug reports: what it holds, and a button that shows its folder. */
export function LogSettings({ operations }: { operations: Operations }) {
  const t = useT();
  const [failed, setFailed] = useState(false);

  const openFolder = () => {
    setFailed(false);
    operations.openLogFolder().catch(() => setFailed(true));
  };

  return (
    <section className="settings-section" aria-labelledby="settings-log">
      <h2 id="settings-log">{t('settings.log')}</h2>
      <p className="note">{t('log.why')}</p>
      <div className="settings-row">
        <span className="muted">{t('log.private')}</span>
        <span className="settings-actions">
          <button type="button" className="button" onClick={openFolder}>
            {t('log.openFolder')}
          </button>
        </span>
      </div>
      {failed && (
        <p className="note" role="alert">
          {t('log.openFailed')}
        </p>
      )}
    </section>
  );
}
