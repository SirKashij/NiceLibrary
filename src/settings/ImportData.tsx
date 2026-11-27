import { useId, useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import type { Operations } from '../operations';
import { useImportReload } from './importReload';

type Run =
  | { state: 'idle' }
  | { state: 'asking' }
  | { state: 'running' }
  | { state: 'refused'; message: MessageKey };

/**
 * Importing an Export File (`importData`): the button first says what the
 * import replaces and what stays, and only on confirmation opens the file
 * dialog. Once the data is replaced, the whole interface starts over and this
 * part says where the data came from; a refused file or a failure is said
 * here, with nothing changed.
 */
export function ImportData({ operations }: { operations: Operations }) {
  const t = useT();
  const { imported, reload } = useImportReload();
  const [run, setRun] = useState<Run>({ state: 'idle' });

  const importData = () => {
    setRun({ state: 'running' });
    operations.importData().then(
      (answer) => {
        if (answer.outcome === 'imported') reload(answer);
        else if (answer.outcome === 'cancelled') setRun({ state: 'idle' });
        else setRun({ state: 'refused', message: answer.outcome === 'newerVersion' ? 'import.newerVersion' : 'import.damaged' });
      },
      () => setRun({ state: 'refused', message: 'import.failed' }),
    );
  };

  return (
    <>
      <div className="settings-row">
        <span className="muted">{t('import.what')}</span>
        <span className="settings-actions">
          <button
            type="button"
            className="button"
            onClick={() => setRun({ state: 'asking' })}
            disabled={run.state === 'running'}
          >
            {t('import.button')}
          </button>
        </span>
      </div>
      {run.state === 'asking' && <ReplaceQuestion onReplace={importData} onKeep={() => setRun({ state: 'idle' })} />}
      {run.state === 'running' && (
        <p className="note" role="status">
          {t('import.running')}
        </p>
      )}
      {run.state === 'refused' && (
        <p className="note" role="alert">
          {t(run.message)}
        </p>
      )}
      {run.state === 'idle' && imported && (
        <p className="note" role="status">
          {t('import.done', { file: imported.file, count: String(imported.localGames) })}
          {imported.localGames > 0 && ` ${t('import.executables')}`}
        </p>
      )}
    </>
  );
}

/** Asks before anything is replaced, saying what the import replaces and what stays. */
function ReplaceQuestion({ onReplace, onKeep }: { onReplace: () => void; onKeep: () => void }) {
  const t = useT();
  const questionId = useId();
  const detailId = useId();
  return (
    <div className="delete-question" role="alertdialog" aria-labelledby={questionId} aria-describedby={detailId}>
      <p id={questionId}>{t('import.question')}</p>
      <div id={detailId}>
        <p>{t('import.replaces')}</p>
        <p>{t('import.lost')}</p>
        <p className="muted">{t('import.keeps')}</p>
      </div>
      <div className="field-row">
        <button type="button" className="button" onClick={onReplace}>
          {t('import.confirm')}
        </button>
        <button type="button" className="button" autoFocus onClick={onKeep}>
          {t('import.keep')}
        </button>
      </div>
    </div>
  );
}
