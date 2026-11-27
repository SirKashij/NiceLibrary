import { useEffect, useId, useRef, useState, type FormEvent } from 'react';
import { useFormat, useT, type MessageKey } from '../i18n';
import type { Game, Operations } from '../operations';

type Direction = 'add' | 'remove';

/** The stored adjustment as the form shows it: which way, whole hours, the minutes left over. */
function fields(minutes: number | null) {
  const amount = Math.abs(minutes ?? 0);
  return {
    direction: (minutes ?? 0) < 0 ? ('remove' as const) : ('add' as const),
    hours: minutes ? String(Math.floor(amount / 60)) : '',
    minutes: minutes ? String(amount % 60) : '',
  };
}

/** A whole number of at least zero, or 0 for an empty field; `null` for anything else. */
function wholeNumber(text: string): number | null {
  const trimmed = text.trim();
  if (trimmed === '') return 0;
  return /^\d+$/.test(trimmed) ? Number(trimmed) : null;
}

/**
 * The Playtime Adjustment of a Local Game on its detail page: the time the
 * person added to or removed from its Playtime by hand, shown as such, and
 * the form to change or clear it. Steam Games have none; this is not shown
 * for them.
 */
export function PlaytimeAdjustment({
  operations,
  game,
  onAdjusted,
}: {
  operations: Operations;
  game: Game;
  /** Hears the Game as the Library now shows it, with its new Playtime. */
  onAdjusted: (game: Game) => void;
}) {
  const t = useT();
  const format = useFormat();
  const headingId = useId();
  // `undefined` while it is read.
  const [adjustment, setAdjustment] = useState<number | null | undefined>(undefined);
  const [form, setForm] = useState(() => fields(null));
  const [problem, setProblem] = useState<MessageKey | null>(null);
  const [saving, setSaving] = useState(false);
  // Only the answer to the latest change counts; an earlier one may answer later.
  const latest = useRef(0);

  useEffect(() => {
    let active = true;
    operations.playtimeAdjustment(game.id).then(
      (minutes) => {
        if (!active) return;
        setAdjustment(minutes);
        setForm(fields(minutes));
      },
      () => active && setProblem('adjustment.loadFailed'),
    );
    return () => {
      active = false;
    };
  }, [operations, game.id]);

  const save = (minutes: number | null) => {
    const id = ++latest.current;
    setProblem(null);
    setSaving(true);
    operations.setPlaytimeAdjustment(game.id, minutes).then(
      (answer) => {
        if (id !== latest.current) return;
        const stored = minutes === 0 ? null : minutes;
        setAdjustment(stored);
        setForm(fields(stored));
        setSaving(false);
        onAdjusted(answer);
      },
      () => {
        if (id !== latest.current) return;
        setProblem('adjustment.saveFailed');
        setSaving(false);
      },
    );
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const hours = wholeNumber(form.hours);
    const minutes = wholeNumber(form.minutes);
    if (hours === null || minutes === null || hours * 60 + minutes === 0) {
      setProblem('adjustment.invalid');
      return;
    }
    const amount = hours * 60 + minutes;
    save(form.direction === 'remove' ? -amount : amount);
  };

  const shown =
    adjustment === undefined
      ? null
      : adjustment === null
        ? t('adjustment.none')
        : t(adjustment > 0 ? 'adjustment.added' : 'adjustment.removed', {
            amount: format.duration(Math.abs(adjustment)),
          });

  return (
    <section className="playtime-adjustment" aria-labelledby={headingId}>
      <h2 id={headingId}>{t('adjustment.heading')}</h2>
      {shown && <p className="note">{shown}</p>}
      <form className="playtime-adjustment-form" onSubmit={submit}>
        <div className="segmented" role="group" aria-label={t('adjustment.direction', { title: game.title })}>
          {(['add', 'remove'] as const).map((direction: Direction) => (
            <button
              key={direction}
              type="button"
              aria-pressed={form.direction === direction}
              onClick={() => setForm({ ...form, direction })}
            >
              {t(direction === 'add' ? 'adjustment.add' : 'adjustment.remove')}
            </button>
          ))}
        </div>
        <label>
          {t('adjustment.hours')}
          <input
            type="number"
            min={0}
            step={1}
            inputMode="numeric"
            value={form.hours}
            onChange={(event) => setForm({ ...form, hours: event.target.value })}
          />
        </label>
        <label>
          {t('adjustment.minutes')}
          <input
            type="number"
            min={0}
            step={1}
            inputMode="numeric"
            value={form.minutes}
            onChange={(event) => setForm({ ...form, minutes: event.target.value })}
          />
        </label>
        <button type="submit" className="button" disabled={saving}>
          {t('adjustment.save')}
        </button>
        {adjustment != null && (
          <button type="button" className="button" disabled={saving} onClick={() => save(null)}>
            {t('adjustment.clear')}
          </button>
        )}
      </form>
      {problem && (
        <p className="note" role="alert">
          {t(problem, { title: game.title })}
        </p>
      )}
    </section>
  );
}
