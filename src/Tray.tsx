import { useEffect, useState } from 'react';
import { useT } from './i18n';
import type { Operations } from './operations';

/**
 * The interface's part of the Tray: gives the Tray icon its texts in the
 * interface language, and on the first close of the window explains that the
 * app keeps running there.
 */
export function Tray({ operations }: { operations: Operations }) {
  const t = useT();
  const [notice, setNotice] = useState(false);
  const tooltip = t('app.name');
  const open = t('tray.open');
  const quit = t('tray.quit');

  useEffect(() => {
    // Without its texts the Tray icon still opens the window; nothing to tell the user.
    operations.showTrayMenu({ tooltip, open, quit }).catch(() => {});
  }, [operations, tooltip, open, quit]);

  useEffect(() => operations.onTrayNotice(() => setNotice(true)), [operations]);

  if (!notice) return null;

  const close = () => {
    setNotice(false);
    // The window is closing; there is nothing left to show a failure in.
    operations.closeWindow().catch(() => {});
  };

  return (
    <div className="tray-notice-backdrop">
      <div
        className="tray-notice"
        role="dialog"
        aria-modal="true"
        aria-labelledby="tray-notice-title"
        aria-describedby="tray-notice-text"
      >
        <h2 id="tray-notice-title">{t('tray.noticeTitle')}</h2>
        <p id="tray-notice-text" className="note">
          {t('tray.notice')}
        </p>
        <button type="button" className="button" autoFocus onClick={close}>
          {t('tray.ok')}
        </button>
      </div>
    </div>
  );
}
