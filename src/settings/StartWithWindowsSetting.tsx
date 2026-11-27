import { useCallback, useEffect, useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import type { Operations } from '../operations';

/**
 * Starting with Windows: whether the app starts hidden in the Tray when the
 * user signs in. Shows what the autostart entry says, read again whenever
 * the window comes back to the front, since the user may change the entry
 * outside the app (in the Task Manager, for instance).
 */
export function StartWithWindowsSetting({ operations }: { operations: Operations }) {
  const t = useT();
  // `undefined` while loading; what Windows will do once the core answered.
  const [on, setOn] = useState<boolean>();
  const [problem, setProblem] = useState<MessageKey>();

  const read = useCallback(
    (isActive: () => boolean) =>
      operations.startsWithWindows().then(
        (starts) => {
          if (isActive()) {
            setOn(starts);
            setProblem((shown) => (shown === 'autostart.loadFailed' ? undefined : shown));
          }
        },
        () => isActive() && setProblem('autostart.loadFailed'),
      ),
    [operations],
  );

  useEffect(() => {
    let active = true;
    const isActive = () => active;
    void read(isActive);
    const readAgain = () => void read(isActive);
    window.addEventListener('focus', readAgain);
    return () => {
      active = false;
      window.removeEventListener('focus', readAgain);
    };
  }, [read]);

  const change = (starts: boolean) => {
    setProblem(undefined);
    operations.setStartWithWindows(starts).then(
      () => setOn(starts),
      () => setProblem('autostart.saveFailed'),
    );
  };

  return (
    <>
      {on !== undefined && (
        <>
          <label className="checkbox-row">
            <input
              type="checkbox"
              aria-describedby="start-with-windows-hint"
              checked={on}
              onChange={(event) => change(event.target.checked)}
            />
            {t('autostart.startWithWindows')}
          </label>
          <p id="start-with-windows-hint" className="muted">
            {t('autostart.hint')}
          </p>
        </>
      )}
      {problem && (
        <p className="note" role="alert">
          {t(problem)}
        </p>
      )}
    </>
  );
}
