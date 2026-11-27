import { useState } from 'react';
import { CoverMatchProposal } from '../CoverMatch';
import { useT, type MessageKey } from '../i18n';
import type { AddLocalGame, Operations, SteamTitle, SuggestedExecutable } from '../operations';

/** A message about adding: `status` for what went well, `alert` for what did not. */
type Notice = { role: 'status' | 'alert'; message: MessageKey; values?: Record<string, string> };

/**
 * A suggested executable as the user ticks and corrects it, with the Cover
 * Match to save with it; `problem` says why adding it was refused.
 */
type Row = SuggestedExecutable & { ticked: boolean; match?: SteamTitle | null; problem?: MessageKey };

/** The folder the user chose and what was found in it. */
type Suggestions = { folder: string; rows: Row[] };

/** What a refusal of the core tells the user about one executable. */
function problem(answer: Exclude<AddLocalGame, { outcome: 'added' | 'alreadyAdded' }>): MessageKey {
  return answer.outcome === 'noTitle' ? 'localGame.noTitle' : 'localGame.notFound';
}

/**
 * Adding several Local Games at once: the user chooses a folder, the app
 * suggests the executables in it with proposed titles, and the user ticks
 * the ones to add and corrects their titles and proposed Cover Matches.
 * Nothing is added until they confirm; executables already in the Library
 * are marked and not offered.
 * `onAdded` hears that the core added Games.
 */
export function AddFromFolderButton({ operations, onAdded }: { operations: Operations; onAdded: () => void }) {
  const t = useT();
  const [choosing, setChoosing] = useState(false);
  const [adding, setAdding] = useState(false);
  const [suggestions, setSuggestions] = useState<Suggestions>();
  const [notice, setNotice] = useState<Notice>();
  // The titles of added Games whose Cover Match could not be saved.
  const [matchesNotSaved, setMatchesNotSaved] = useState<string[]>([]);

  const choose = () => {
    if (choosing) return;
    setChoosing(true);
    setNotice(undefined);
    operations
      .suggestExecutablesInFolder()
      .then(
        (answer) => {
          if (answer.outcome === 'cancelled') return;
          if (answer.executables.length === 0)
            return setNotice({ role: 'status', message: 'addFromFolder.none', values: { folder: answer.folder } });
          setSuggestions({
            folder: answer.folder,
            rows: answer.executables.map((suggested) => ({ ...suggested, ticked: false })),
          });
        },
        () => setNotice({ role: 'alert', message: 'addFromFolder.dialogFailed' }),
      )
      .finally(() => setChoosing(false));
  };

  const change = (executable: string, change: Partial<Row>) =>
    setSuggestions(
      (current) =>
        current && {
          ...current,
          rows: current.rows.map((row) => (row.executable === executable ? { ...row, ...change } : row)),
        },
    );

  const add = async (current: Suggestions) => {
    if (adding) return;
    setAdding(true);
    setNotice(undefined);
    setMatchesNotSaved([]);
    const notSaved: string[] = [];
    let count = 0;
    const rows: Row[] = [];
    for (const row of current.rows) {
      if (row.alreadyAdded || !row.ticked) {
        rows.push(row);
        continue;
      }
      try {
        const answer = await operations.addLocalGame(row.executable, row.title);
        if (answer.outcome === 'added') {
          count += 1;
          if (row.match) {
            // Saved before the Library loads the new Game's Cover.
            const title = answer.game.title;
            await operations.setCoverMatch(answer.game.id, row.match.appId).catch(() => notSaved.push(title));
          }
          rows.push({ ...row, title: answer.game.title, alreadyAdded: true, ticked: false, problem: undefined });
        } else if (answer.outcome === 'alreadyAdded') {
          rows.push({ ...row, title: answer.title, alreadyAdded: true, ticked: false, problem: undefined });
        } else {
          rows.push({ ...row, problem: problem(answer) });
        }
      } catch {
        rows.push({ ...row, problem: 'localGame.addFailed' });
      }
    }
    setAdding(false);
    setMatchesNotSaved(notSaved);
    setSuggestions(rows.some((row) => row.problem) ? { ...current, rows } : undefined);
    if (count > 0) {
      setNotice({ role: 'status', message: 'addFromFolder.added', values: { count: String(count) } });
      onAdded();
    }
  };

  const cancel = () => {
    setSuggestions(undefined);
    setNotice(undefined);
    setMatchesNotSaved([]);
  };

  return (
    <div className="add-local-game">
      {!suggestions && (
        <button type="button" className="button" disabled={choosing} onClick={choose}>
          {t('addFromFolder.add')}
        </button>
      )}
      {choosing && (
        <p className="note" role="status">
          {t('addFromFolder.choosing')}
        </p>
      )}
      {suggestions && (
        <form
          className="local-game-form"
          aria-label={t('addFromFolder.suggestions', { folder: suggestions.folder })}
          onSubmit={(event) => {
            event.preventDefault();
            void add(suggestions);
          }}
        >
          <p className="muted">{t('addFromFolder.hint')}</p>
          <ul className="suggested-executables">
            {suggestions.rows.map((row) => (
              <li key={row.executable} className="suggested-executable">
                {row.alreadyAdded ? (
                  <div className="field-row">
                    <span>{row.title}</span>
                    <span className="muted">{t('addFromFolder.alreadyAdded')}</span>
                  </div>
                ) : (
                  <div className="field-row">
                    <input
                      type="checkbox"
                      aria-label={t('addFromFolder.tick', { title: row.title.trim() || row.executable })}
                      checked={row.ticked}
                      onChange={(event) => change(row.executable, { ticked: event.target.checked })}
                    />
                    <input
                      type="text"
                      autoComplete="off"
                      aria-label={t('addFromFolder.titleOf', { executable: row.executable })}
                      value={row.title}
                      onChange={(event) => change(row.executable, { title: event.target.value })}
                    />
                  </div>
                )}
                <span className="muted local-game-executable">{row.executable}</span>
                {!row.alreadyAdded && (
                  <CoverMatchProposal
                    operations={operations}
                    title={row.title}
                    onChange={(match) => change(row.executable, { match })}
                  />
                )}
                {row.problem && (
                  <p className="note" role="alert">
                    {t(row.problem)}
                  </p>
                )}
              </li>
            ))}
          </ul>
          <div className="field-row">
            <button
              type="submit"
              className="button"
              disabled={adding || !suggestions.rows.some((row) => row.ticked && !row.alreadyAdded)}
            >
              {t('addFromFolder.confirm')}
            </button>
            <button type="button" className="button" onClick={cancel}>
              {t('addFromFolder.cancel')}
            </button>
          </div>
        </form>
      )}
      {notice && (
        <p className="note" role={notice.role}>
          {t(notice.message, notice.values)}
        </p>
      )}
      {matchesNotSaved.map((title) => (
        <p key={title} className="note" role="alert">
          {t('coverMatch.notSaved', { title })}
        </p>
      ))}
    </div>
  );
}
