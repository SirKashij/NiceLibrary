import { useEffect, useId, useState } from 'react';
import { useT, type MessageKey } from './i18n';
import type { Game, Operations } from './operations';

/** The fields as the user edits them; an empty Working Directory is none. */
type Draft = { arguments: string; workingDirectory: string };

/** A message about saving: `status` for what went well, `alert` for what did not. */
type Notice = { role: 'status' | 'alert'; message: MessageKey };

/**
 * The Launch Options of a Local Game on its Detail Page: its Launch
 * Arguments and its Working Directory, as stored, for the user to change.
 */
export function LaunchOptionsForm({ operations, game }: { operations: Operations; game: Game }) {
  const t = useT();
  const id = useId();
  const [draft, setDraft] = useState<Draft | 'loading' | 'none' | 'failed'>('loading');
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<Notice>();

  useEffect(() => {
    let active = true;
    operations.launchOptions(game.id).then(
      (options) =>
        active &&
        setDraft(options ? { arguments: options.arguments, workingDirectory: options.workingDirectory ?? '' } : 'none'),
      () => active && setDraft('failed'),
    );
    return () => {
      active = false;
    };
  }, [operations, game.id]);

  // `none` once the Game is no longer a Local Game in the Library; its page says so.
  if (draft === 'loading' || draft === 'none') return null;
  if (draft === 'failed')
    return (
      <p className="note" role="alert">
        {t('launchOptions.loadFailed')}
      </p>
    );

  const edit = (change: Partial<Draft>) => {
    setDraft({ ...draft, ...change });
    setNotice(undefined);
  };

  const save = () => {
    if (saving) return;
    setSaving(true);
    setNotice(undefined);
    const workingDirectory = draft.workingDirectory.trim();
    operations
      .setLaunchOptions(game.id, { arguments: draft.arguments, workingDirectory: workingDirectory || null })
      .then(
        () => setNotice({ role: 'status', message: 'launchOptions.saved' }),
        () => setNotice({ role: 'alert', message: 'launchOptions.saveFailed' }),
      )
      .finally(() => setSaving(false));
  };

  return (
    <form
      className="launch-options"
      aria-labelledby={`${id}-title`}
      onSubmit={(event) => {
        event.preventDefault();
        save();
      }}
    >
      <h2 id={`${id}-title`}>{t('launchOptions.title')}</h2>
      <label htmlFor={`${id}-arguments`}>{t('launchOptions.arguments')}</label>
      <input
        id={`${id}-arguments`}
        type="text"
        autoComplete="off"
        spellCheck={false}
        aria-describedby={`${id}-arguments-hint`}
        value={draft.arguments}
        onChange={(event) => edit({ arguments: event.target.value })}
      />
      <p id={`${id}-arguments-hint`} className="muted">
        {t('launchOptions.argumentsHint')}
      </p>
      <label htmlFor={`${id}-folder`}>{t('launchOptions.workingDirectory')}</label>
      <input
        id={`${id}-folder`}
        type="text"
        autoComplete="off"
        spellCheck={false}
        aria-describedby={`${id}-folder-hint`}
        value={draft.workingDirectory}
        onChange={(event) => edit({ workingDirectory: event.target.value })}
      />
      <p id={`${id}-folder-hint`} className="muted">
        {t('launchOptions.workingDirectoryHint')}
      </p>
      <div>
        <button type="submit" className="button" disabled={saving}>
          {t('launchOptions.save')}
        </button>
      </div>
      {notice && (
        <p className="note" role={notice.role}>
          {t(notice.message)}
        </p>
      )}
    </form>
  );
}
