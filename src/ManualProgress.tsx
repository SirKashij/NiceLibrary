import { useId, useState } from 'react';
import { useT, type MessageKey } from './i18n';
import type { Game, Operations } from './operations';

/**
 * `typed` as a Manual Progress to hand the core: `null` for an empty field,
 * which clears it, a whole number as typed, or `undefined` for anything else.
 * Whether the number lies between 0 and 100 is the core's to say.
 */
function percentIn(typed: string): number | null | undefined {
  const text = typed.trim();
  if (text === '') return null;
  return /^\d+$/.test(text) ? Number(text) : undefined;
}

/**
 * Setting the Manual Progress of a Local Game by hand, from 0 to 100, and
 * clearing it again. `onSet` hears the Game as the core answers it after a
 * change. Steam Games offer no Manual Progress, so this is shown for Local
 * Games only.
 */
export function ManualProgressForm({
  operations,
  game,
  onSet,
}: {
  operations: Operations;
  game: Game;
  onSet: (game: Game) => void;
}) {
  const t = useT();
  const headingId = useId();
  const fieldId = useId();
  const hintId = useId();
  const [typed, setTyped] = useState(game.progress === null ? '' : String(game.progress));
  const [saving, setSaving] = useState(false);
  const [failure, setFailure] = useState<MessageKey>();

  const save = (percent: number | null | undefined) => {
    if (saving) return;
    if (percent === undefined) return setFailure('progress.invalid');
    setSaving(true);
    setFailure(undefined);
    operations
      .setManualProgress(game.id, percent)
      .then(
        (answer) => {
          if (answer.outcome !== 'set') return setFailure('progress.invalid');
          setTyped(answer.game.progress === null ? '' : String(answer.game.progress));
          onSet(answer.game);
        },
        () => setFailure('progress.saveFailed'),
      )
      .finally(() => setSaving(false));
  };

  return (
    <section className="manual-progress" aria-labelledby={headingId}>
      <h2 id={headingId}>{t('progress.manual')}</h2>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          save(percentIn(typed));
        }}
      >
        <label htmlFor={fieldId} className="visually-hidden">
          {t('progress.manualField')}
        </label>
        <div className="field-row">
          <input
            id={fieldId}
            type="text"
            inputMode="numeric"
            autoComplete="off"
            className="manual-progress-field"
            aria-describedby={hintId}
            aria-invalid={failure === 'progress.invalid'}
            value={typed}
            onChange={(event) => setTyped(event.target.value)}
          />
          <span aria-hidden="true">%</span>
          <button type="submit" className="button" disabled={saving}>
            {t('progress.save')}
          </button>
          {game.progress !== null && (
            <button type="button" className="button" disabled={saving} onClick={() => save(null)}>
              {t('progress.clear')}
            </button>
          )}
        </div>
        <p id={hintId} className="muted">
          {t('progress.manualHint')}
        </p>
      </form>
      {failure && (
        <p className="note" role="alert">
          {t(failure)}
        </p>
      )}
    </section>
  );
}
