import { useEffect, useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import type { Operations } from '../operations';
import { StartWithWindowsSetting } from './StartWithWindowsSetting';

/**
 * The Window settings: whether closing the window keeps the app running in
 * the Tray (close-to-tray), and whether the app starts with Windows.
 */
export function WindowSettings({ operations }: { operations: Operations }) {
  const t = useT();
  // `undefined` while loading; what is saved once the core answered.
  const [closeToTray, setCloseToTray] = useState<boolean>();
  const [problem, setProblem] = useState<MessageKey>();

  useEffect(() => {
    let active = true;
    operations.closeToTray().then(
      (on) => active && setCloseToTray(on),
      () => active && setProblem('window.loadFailed'),
    );
    return () => {
      active = false;
    };
  }, [operations]);

  const change = (on: boolean) => {
    setProblem(undefined);
    operations.setCloseToTray(on).then(
      () => setCloseToTray(on),
      () => setProblem('window.saveFailed'),
    );
  };

  return (
    <section className="settings-section" aria-labelledby="settings-window">
      <h2 id="settings-window">{t('settings.window')}</h2>
      {closeToTray !== undefined && (
        <>
          <label className="checkbox-row">
            <input
              type="checkbox"
              aria-describedby="close-to-tray-hint"
              checked={closeToTray}
              onChange={(event) => change(event.target.checked)}
            />
            {t('window.closeToTray')}
          </label>
          <p id="close-to-tray-hint" className="muted">
            {t(closeToTray ? 'window.closeToTrayOn' : 'window.closeToTrayOff')}
          </p>
        </>
      )}
      {problem && (
        <p className="note" role="alert">
          {t(problem)}
        </p>
      )}
      <StartWithWindowsSetting operations={operations} />
    </section>
  );
}
