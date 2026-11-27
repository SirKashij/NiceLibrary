import { useId, useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import type { EditLocalGame, Game, LocalGame, Operations } from '../operations';

/** A message about editing or removing: `status` for what went well, `alert` for what did not. */
type Notice = { role: 'status' | 'alert'; message: MessageKey; title?: string };

/** What the actions show besides their buttons. */
type Mode = { kind: 'idle' } | { kind: 'editing'; draft: LocalGame } | { kind: 'confirmingRemoval' };

/** What an answer of the core other than `edited` tells the user. */
function refusal(answer: Exclude<EditLocalGame, { outcome: 'edited' }>): Notice {
  switch (answer.outcome) {
    case 'alreadyAdded':
      return { role: 'alert', message: 'editGame.alreadyAdded', title: answer.title };
    case 'noTitle':
      return { role: 'alert', message: 'editGame.noTitle' };
    case 'notFound':
      return { role: 'alert', message: 'editGame.notFound' };
  }
}

/**
 * Editing and removing a Local Game, on its detail page: the user changes its
 * title and executable, or removes it from the Library after confirming.
 * Only for Local Games; Steam Games come from Steam and are not edited here.
 * `onEdited` hears each change the core saved, `onRemoved` the removal.
 */
export function LocalGameActions({
  operations,
  game,
  onEdited,
  onRemoved,
}: {
  operations: Operations;
  game: Game;
  onEdited: () => void;
  onRemoved: (game: Game) => void;
}) {
  const t = useT();
  const [mode, setMode] = useState<Mode>({ kind: 'idle' });
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice>();

  const edit = () => {
    if (busy) return;
    setBusy(true);
    setNotice(undefined);
    operations
      .localGame(game.id)
      .then(
        (stored) => {
          if (stored) setMode({ kind: 'editing', draft: stored });
          else setNotice({ role: 'alert', message: 'editGame.loadFailed' });
        },
        () => setNotice({ role: 'alert', message: 'editGame.loadFailed' }),
      )
      .finally(() => setBusy(false));
  };

  const chooseExecutable = (draft: LocalGame) => {
    if (busy) return;
    setBusy(true);
    setNotice({ role: 'status', message: 'editGame.choosing' });
    operations
      .chooseExecutableFor(game.id)
      .then(
        (choice) => {
          setNotice(undefined);
          if (choice.outcome === 'chosen') setMode({ kind: 'editing', draft: { ...draft, executable: choice.executable } });
          else if (choice.outcome === 'alreadyAdded')
            setNotice({ role: 'alert', message: 'editGame.alreadyAdded', title: choice.title });
        },
        () => setNotice({ role: 'alert', message: 'editGame.dialogFailed' }),
      )
      .finally(() => setBusy(false));
  };

  const save = (draft: LocalGame) => {
    if (busy) return;
    setBusy(true);
    setNotice(undefined);
    operations
      .editLocalGame(game.id, draft.title, draft.executable)
      .then(
        (answer) => {
          if (answer.outcome !== 'edited') return setNotice(refusal(answer));
          setMode({ kind: 'idle' });
          setNotice({ role: 'status', message: 'editGame.saved' });
          onEdited();
        },
        () => setNotice({ role: 'alert', message: 'editGame.saveFailed' }),
      )
      .finally(() => setBusy(false));
  };

  const remove = () => {
    if (busy) return;
    setBusy(true);
    setNotice(undefined);
    operations.removeLocalGame(game.id).then(
      () => onRemoved(game),
      () => {
        setBusy(false);
        setMode({ kind: 'idle' });
        setNotice({ role: 'alert', message: 'editGame.removeFailed', title: game.title });
      },
    );
  };

  return (
    <div className="local-game-actions">
      {mode.kind !== 'editing' && (
        <div className="field-row">
          <button
            type="button"
            className="button"
            aria-label={t('editGame.edit', { title: game.title })}
            disabled={busy}
            onClick={edit}
          >
            {t('editGame.editShort')}
          </button>
          <button
            type="button"
            className="button"
            aria-label={t('editGame.remove', { title: game.title })}
            disabled={busy}
            onClick={() => {
              setNotice(undefined);
              setMode({ kind: 'confirmingRemoval' });
            }}
          >
            {t('editGame.removeShort')}
          </button>
        </div>
      )}
      {mode.kind === 'editing' && (
        <EditForm
          game={game}
          draft={mode.draft}
          busy={busy}
          onChange={(draft) => setMode({ kind: 'editing', draft })}
          onChooseExecutable={() => chooseExecutable(mode.draft)}
          onSave={() => save(mode.draft)}
          onCancel={() => {
            setNotice(undefined);
            setMode({ kind: 'idle' });
          }}
        />
      )}
      {mode.kind === 'confirmingRemoval' && (
        <RemoveQuestion
          title={game.title}
          busy={busy}
          onRemove={remove}
          onKeep={() => setMode({ kind: 'idle' })}
        />
      )}
      {notice && (
        <p className="note" role={notice.role}>
          {t(notice.message, notice.title === undefined ? undefined : { title: notice.title })}
        </p>
      )}
    </div>
  );
}

function EditForm({
  game,
  draft,
  busy,
  onChange,
  onChooseExecutable,
  onSave,
  onCancel,
}: {
  game: Game;
  draft: LocalGame;
  busy: boolean;
  onChange: (draft: LocalGame) => void;
  onChooseExecutable: () => void;
  onSave: () => void;
  onCancel: () => void;
}) {
  const t = useT();
  const titleId = useId();
  return (
    <form
      className="local-game-form"
      aria-label={t('editGame.edit', { title: game.title })}
      onSubmit={(event) => {
        event.preventDefault();
        onSave();
      }}
    >
      <label htmlFor={titleId}>{t('editGame.title')}</label>
      <input
        id={titleId}
        type="text"
        autoComplete="off"
        autoFocus
        value={draft.title}
        onChange={(event) => onChange({ ...draft, title: event.target.value })}
      />
      <div className="settings-row">
        <span>{t('editGame.executable')}</span>
        <span className="muted local-game-executable">{draft.executable}</span>
      </div>
      <div className="field-row">
        <button type="button" className="button" disabled={busy} onClick={onChooseExecutable}>
          {t('editGame.chooseExecutable')}
        </button>
        <button type="submit" className="button" disabled={busy}>
          {t('editGame.save')}
        </button>
        <button type="button" className="button" onClick={onCancel}>
          {t('editGame.cancel')}
        </button>
      </div>
    </form>
  );
}

/** Asks before a Local Game is removed, and says that its files stay. */
function RemoveQuestion({
  title,
  busy,
  onRemove,
  onKeep,
}: {
  title: string;
  busy: boolean;
  onRemove: () => void;
  onKeep: () => void;
}) {
  const t = useT();
  const questionId = useId();
  const detailId = useId();
  return (
    <div className="delete-question" role="alertdialog" aria-labelledby={questionId} aria-describedby={detailId}>
      <p id={questionId}>{t('editGame.removeQuestion', { title })}</p>
      <p id={detailId} className="muted">
        {t('editGame.removeKeepsFiles')}
      </p>
      <div className="field-row">
        <button type="button" className="button" disabled={busy} onClick={onRemove}>
          {t('editGame.confirmRemove')}
        </button>
        <button type="button" className="button" autoFocus onClick={onKeep}>
          {t('editGame.keep')}
        </button>
      </div>
    </div>
  );
}
