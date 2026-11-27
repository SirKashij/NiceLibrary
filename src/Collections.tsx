import { useEffect, useId, useState } from 'react';
import { useT, type MessageKey } from './i18n';
import type { Collection, CollectionNaming, Operations } from './operations';

/** What the user is doing with the Collections right now, if anything. */
type Editing =
  | { kind: 'create' }
  | { kind: 'rename'; collection: Collection }
  | { kind: 'delete'; collection: Collection };

/** Why the last change did not take, with the name it was about. */
type Problem = { message: MessageKey; name?: string };

/** What a naming that changed nothing answers, for the user. */
const NAMING_PROBLEMS: Record<Exclude<CollectionNaming['outcome'], 'named'>, MessageKey> = {
  emptyName: 'collections.emptyName',
  nameTaken: 'collections.nameTaken',
};

/**
 * The Collections in the sidebar, each with how many Games are in it, and
 * creating, renaming and deleting them. The list always comes from the
 * core, which orders it and decides which names are allowed.
 */
export function Collections({ operations }: { operations: Operations }) {
  const t = useT();
  const headingId = useId();
  const [collections, setCollections] = useState<Collection[]>();
  const [loadFailed, setLoadFailed] = useState(false);
  const [editing, setEditing] = useState<Editing>();
  const [problem, setProblem] = useState<Problem>();

  const reload = () =>
    operations.collections().then(
      (answer) => {
        setCollections(answer);
        setLoadFailed(false);
      },
      () => setLoadFailed(true),
    );

  useEffect(() => {
    let active = true;
    operations.collections().then(
      (answer) => active && setCollections(answer),
      () => active && setLoadFailed(true),
    );
    return () => {
      active = false;
    };
  }, [operations]);

  /** Begins `next`, or with nothing ends what the user was doing. */
  const start = (next?: Editing) => {
    setEditing(next);
    setProblem(undefined);
  };
  const stop = () => start();

  /** Saves a name; the form stays open with the reason when the core refuses it. */
  const name = (naming: Promise<CollectionNaming>, attempted: string) =>
    naming.then(
      (answer) => {
        if (answer.outcome === 'named') {
          setEditing(undefined);
          setProblem(undefined);
          return reload();
        }
        setProblem({ message: NAMING_PROBLEMS[answer.outcome], name: attempted.trim() });
      },
      () => setProblem({ message: 'collections.saveFailed' }),
    );

  const remove = (collection: Collection) =>
    operations.deleteCollection(collection.id).then(
      () => {
        setEditing(undefined);
        return reload();
      },
      () => {
        setEditing(undefined);
        setProblem({ message: 'collections.deleteFailed' });
      },
    );

  return (
    <section className="collections" aria-labelledby={headingId}>
      <h2 id={headingId} className="sidebar-heading">
        {t('collections.title')}
      </h2>
      {loadFailed && (
        <p className="note" role="alert">
          {t('collections.loadFailed')}
        </p>
      )}
      {collections?.length === 0 && <p className="muted">{t('collections.none')}</p>}
      {collections && collections.length > 0 && (
        <ul className="collection-list" aria-labelledby={headingId}>
          {collections.map((collection) => (
            <li key={collection.id} className="collection">
              {editing?.kind === 'rename' && editing.collection.id === collection.id ? (
                <NameForm
                  label={t('collections.renameName', { name: collection.name })}
                  initial={collection.name}
                  submit={t('collections.save')}
                  onSubmit={(draft) => name(operations.renameCollection(collection.id, draft), draft)}
                  onCancel={stop}
                />
              ) : (
                <div className="collection-row">
                  <span className="collection-name">{collection.name}</span>
                  <span className="collection-count">
                    <span aria-hidden="true">{collection.gameCount}</span>
                    <span className="visually-hidden">
                      {collection.gameCount === 1
                        ? t('collections.oneGame')
                        : t('collections.games', { count: String(collection.gameCount) })}
                    </span>
                  </span>
                  <span className="collection-actions">
                    <button
                      type="button"
                      className="text-button"
                      aria-label={t('collections.rename', { name: collection.name })}
                      onClick={() => start({ kind: 'rename', collection })}
                    >
                      <Icon path="M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4" />
                    </button>
                    <button
                      type="button"
                      className="text-button"
                      aria-label={t('collections.delete', { name: collection.name })}
                      onClick={() => start({ kind: 'delete', collection })}
                    >
                      <Icon path="M6 6l12 12M18 6L6 18" />
                    </button>
                  </span>
                </div>
              )}
              {editing?.kind === 'delete' && editing.collection.id === collection.id && (
                <DeleteQuestion
                  name={collection.name}
                  onDelete={() => void remove(collection)}
                  onKeep={stop}
                />
              )}
            </li>
          ))}
        </ul>
      )}
      {editing?.kind === 'create' ? (
        <NameForm
          label={t('collections.newName')}
          initial=""
          submit={t('collections.create')}
          onSubmit={(draft) => name(operations.createCollection(draft), draft)}
          onCancel={stop}
        />
      ) : (
        <button type="button" className="button" onClick={() => start({ kind: 'create' })}>
          {t('collections.new')}
        </button>
      )}
      {problem && (
        <p className="note" role="alert">
          {t(problem.message, problem.name === undefined ? undefined : { name: problem.name })}
        </p>
      )}
    </section>
  );
}

/** A line icon for a button that names itself with `aria-label`. */
function Icon({ path }: { path: string }) {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d={path} />
    </svg>
  );
}

/** A name to save: for a new Collection or in place of a Collection's name. */
function NameForm({
  label,
  initial,
  submit,
  onSubmit,
  onCancel,
}: {
  label: string;
  initial: string;
  submit: string;
  onSubmit: (draft: string) => Promise<unknown>;
  onCancel: () => void;
}) {
  const t = useT();
  const id = useId();
  const [draft, setDraft] = useState(initial);
  const [saving, setSaving] = useState(false);
  return (
    <form
      className="collection-form"
      onSubmit={(event) => {
        event.preventDefault();
        if (saving) return;
        setSaving(true);
        void onSubmit(draft).finally(() => setSaving(false));
      }}
    >
      <label htmlFor={id} className="visually-hidden">
        {label}
      </label>
      <input
        id={id}
        type="text"
        autoComplete="off"
        // The form opens because the user asked to type a name.
        autoFocus
        value={draft}
        placeholder={label}
        onChange={(event) => setDraft(event.target.value)}
      />
      <div className="field-row">
        <button type="submit" className="button" disabled={saving}>
          {submit}
        </button>
        <button type="button" className="button" onClick={onCancel}>
          {t('collections.cancel')}
        </button>
      </div>
    </form>
  );
}

/** Asks before a Collection is deleted, and says that its Games stay. */
function DeleteQuestion({ name, onDelete, onKeep }: { name: string; onDelete: () => void; onKeep: () => void }) {
  const t = useT();
  const questionId = useId();
  const detailId = useId();
  return (
    <div className="delete-question" role="alertdialog" aria-labelledby={questionId} aria-describedby={detailId}>
      <p id={questionId}>{t('collections.deleteQuestion', { name })}</p>
      <p id={detailId} className="muted">
        {t('collections.deleteKeepsGames')}
      </p>
      <div className="field-row">
        <button type="button" className="button" onClick={onDelete}>
          {t('collections.confirmDelete')}
        </button>
        <button type="button" className="button" autoFocus onClick={onKeep}>
          {t('collections.keep')}
        </button>
      </div>
    </div>
  );
}
