import { useState } from 'react';
import { useT } from '../i18n';
import type { Operations } from '../operations';
import { ImportData } from './ImportData';

type Export = { state: 'idle' } | { state: 'running' } | { state: 'exported'; file: string } | { state: 'failed' };

/**
 * The backup: what an Export File holds, and a button that asks where to
 * save one and writes it (`exportData`). Cancelling the dialog says nothing.
 * Below it, importing one (see `ImportData`).
 */
export function ExportSettings({ operations }: { operations: Operations }) {
  const t = useT();
  const [run, setRun] = useState<Export>({ state: 'idle' });

  const exportData = () => {
    setRun({ state: 'running' });
    operations.exportData().then(
      (answer) => setRun(answer.outcome === 'exported' ? { state: 'exported', file: answer.file } : { state: 'idle' }),
      () => setRun({ state: 'failed' }),
    );
  };

  return (
    <section className="settings-section" aria-labelledby="settings-export">
      <h2 id="settings-export">{t('settings.export')}</h2>
      <p className="note">{t('export.what')}</p>
      <div className="settings-row">
        <span className="muted">{t('export.noKey')}</span>
        <span className="settings-actions">
          <button type="button" className="button" onClick={exportData} disabled={run.state === 'running'}>
            {t('export.button')}
          </button>
        </span>
      </div>
      {run.state === 'running' && (
        <p className="note" role="status">
          {t('export.running')}
        </p>
      )}
      {run.state === 'exported' && (
        <p className="note" role="status">
          {t('export.done', { file: run.file })}
        </p>
      )}
      {run.state === 'failed' && (
        <p className="note" role="alert">
          {t('export.failed')}
        </p>
      )}
      <ImportData operations={operations} />
    </section>
  );
}
