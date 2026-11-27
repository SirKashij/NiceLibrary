import { useEffect, useId, useRef, useState } from 'react';
import { useT, type MessageKey } from './i18n';
import type { Collection, Operations } from './operations';

/** Every Collection and which of them the Game is in, as the core answered last. */
type Membership = { collections: Collection[]; memberOf: Set<number> };

/**
 * On a Game's detail page: every Collection with a checkbox, checked for
 * those the Game is in. Checking one puts the Game into it, unchecking takes
 * it out; the boxes always show what the core answered, read again whenever
 * `changes` changes.
 */
export function GameCollections({
  operations,
  gameId,
  title,
  changes,
  onChanged,
}: {
  operations: Operations;
  gameId: string;
  /** The Game's title, for the messages. */
  title: string;
  /** Changes whenever the Collections or what is in them may have changed. */
  changes: number;
  /** Hears each change made here, once the core took it. */
  onChanged: () => void;
}) {
  const t = useT();
  const legendId = useId();
  const [membership, setMembership] = useState<Membership>();
  const [problem, setProblem] = useState<MessageKey>();
  // The Collections whose change is on its way to the core; their boxes wait.
  const [changing, setChanging] = useState<ReadonlySet<number>>(new Set());
  // Only the answer to the latest reading counts; an earlier one may answer later.
  const latestReading = useRef(0);

  useEffect(() => {
    const reading = ++latestReading.current;
    Promise.all([operations.collections(), operations.gameCollections(gameId)]).then(
      ([collections, ids]) => {
        if (reading !== latestReading.current) return;
        setMembership({ collections, memberOf: new Set(ids) });
      },
      () => reading === latestReading.current && setProblem('gameCollections.loadFailed'),
    );
  }, [operations, gameId, changes]);

  const toggle = (collection: Collection, member: boolean) => {
    if (changing.has(collection.id)) return;
    setProblem(undefined);
    setChanging((ids) => new Set(ids).add(collection.id));
    const change = member
      ? operations.addGameToCollection(collection.id, gameId)
      : operations.removeGameFromCollection(collection.id, gameId);
    change
      .then(onChanged, () => setProblem('gameCollections.changeFailed'))
      .finally(() =>
        setChanging((ids) => {
          const next = new Set(ids);
          next.delete(collection.id);
          return next;
        }),
      );
  };

  return (
    <fieldset className="game-collections" aria-labelledby={legendId}>
      <legend id={legendId}>{t('gameCollections.title')}</legend>
      {membership?.collections.length === 0 && <p className="muted">{t('gameCollections.none')}</p>}
      {membership && membership.collections.length > 0 && (
        <ul className="game-collection-list">
          {membership.collections.map((collection) => (
            <li key={collection.id}>
              <label className="toolbar-toggle">
                <input
                  type="checkbox"
                  checked={membership.memberOf.has(collection.id)}
                  aria-busy={changing.has(collection.id) || undefined}
                  onChange={(event) => toggle(collection, event.target.checked)}
                />
                {collection.name}
              </label>
            </li>
          ))}
        </ul>
      )}
      {problem && (
        <p className="note" role="alert">
          {t(problem, { title })}
        </p>
      )}
    </fieldset>
  );
}
