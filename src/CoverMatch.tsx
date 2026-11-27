import { useEffect, useId, useRef, useState } from 'react';
import { useT, type MessageKey } from './i18n';
import type { Game, Operations, SteamTitle } from './operations';

/**
 * The Steam title as the cover protocol knows it: the cover of `steam:<app id>`
 * is that title's Steam Cover, whether or not it is in the Library.
 */
function steamTitleGame(title: SteamTitle): Game {
  return {
    id: `steam:${title.appId}`,
    title: title.name,
    source: 'steam',
    installed: false,
    playtimeMinutes: null,
    lastPlayed: null,
    status: 'unplayed',
    favorite: false,
    progress: null,
  };
}

/** The Steam Cover of a title, small, as a preview of a Cover Match; nothing when Steam has none. */
function SteamCoverPreview({ operations, title }: { operations: Operations; title: SteamTitle }) {
  const t = useT();
  const src = operations.coverUrl(steamTitleGame(title));
  const [failedSrc, setFailedSrc] = useState<string>();
  if (failedSrc === src) return null;
  return (
    <img
      className="cover cover-match-preview"
      src={src}
      alt={t('library.cover', { title: title.name })}
      decoding="async"
      onError={() => setFailedSrc(src)}
    />
  );
}

/**
 * A search through the Steam App List, as the user types; `onPick` hears
 * the title they pick, `onClose` that they want none.
 */
function SteamTitleSearch({
  operations,
  onPick,
  onClose,
}: {
  operations: Operations;
  onPick: (title: SteamTitle) => void;
  onClose: () => void;
}) {
  const t = useT();
  const id = useId();
  const [typed, setTyped] = useState('');
  // The titles found for what was typed, once the core answered.
  const [found, setFound] = useState<{ typed: string; titles: SteamTitle[] }>();
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (typed.trim() === '') return;
    let active = true;
    operations.findSteamTitles(typed).then(
      (titles) => {
        if (!active) return;
        setFailed(false);
        setFound({ typed, titles });
      },
      () => active && setFailed(true),
    );
    return () => {
      active = false;
    };
  }, [operations, typed]);

  const titles = typed.trim() !== '' && found?.typed === typed ? found.titles : undefined;
  return (
    <div className="cover-match-search">
      <label htmlFor={id}>{t('coverMatch.searchLabel')}</label>
      <div className="field-row">
        <input
          id={id}
          type="search"
          autoComplete="off"
          autoFocus
          value={typed}
          onChange={(event) => setTyped(event.target.value)}
          onKeyDown={(event) => {
            // Enter in the search picks nothing, and must not add the Game.
            if (event.key === 'Enter') event.preventDefault();
          }}
        />
        <button type="button" className="button" onClick={onClose}>
          {t('coverMatch.closeSearch')}
        </button>
      </div>
      {failed && (
        <p className="note" role="alert">
          {t('coverMatch.searchFailed')}
        </p>
      )}
      {titles && titles.length === 0 && <p className="muted">{t('coverMatch.noResults')}</p>}
      {titles && titles.length > 0 && (
        <ul className="cover-match-results" aria-label={t('coverMatch.results')}>
          {titles.map((title) => (
            <li key={title.appId}>
              <button type="button" className="cover-match-result" onClick={() => onPick(title)}>
                {title.name}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * What a Cover Match shows: the matched title with its Steam Cover, or what
 * the Game shows without one, with the ways to change it.
 */
function CoverMatchView({
  operations,
  match,
  without,
  removeLabel,
  preview = false,
  disabled = false,
  onPick,
  onRemove,
}: {
  operations: Operations;
  match: SteamTitle | null;
  /** What is said when there is no match; nothing is said when it is undefined. */
  without?: MessageKey;
  removeLabel: MessageKey;
  /** Whether the matched title's Steam Cover is shown with it. */
  preview?: boolean;
  disabled?: boolean;
  onPick: (title: SteamTitle) => void;
  onRemove: () => void;
}) {
  const t = useT();
  const [searching, setSearching] = useState(false);
  return (
    <div className="cover-match">
      {match ? (
        <div className="cover-match-title">
          {preview && <SteamCoverPreview operations={operations} title={match} />}
          <p>{t('coverMatch.matched', { name: match.name })}</p>
        </div>
      ) : (
        without && <p className="muted">{t(without)}</p>
      )}
      {!searching && (
        <div className="field-row">
          <button type="button" className="button" disabled={disabled} onClick={() => setSearching(true)}>
            {t(match ? 'coverMatch.chooseOther' : 'coverMatch.choose')}
          </button>
          {match && (
            <button type="button" className="button" disabled={disabled} onClick={onRemove}>
              {t(removeLabel)}
            </button>
          )}
        </div>
      )}
      {searching && (
        <SteamTitleSearch
          operations={operations}
          onPick={(title) => {
            setSearching(false);
            onPick(title);
          }}
          onClose={() => setSearching(false)}
        />
      )}
    </div>
  );
}

/**
 * The Cover Match proposed while a Local Game is being added, before it is
 * saved: the Steam title that best matches `title`, which the user keeps,
 * replaces with another or declines. It follows the title as the user
 * corrects it, until they pick or decline themselves. `onChange` hears the
 * match to save with the Game, `null` for none.
 */
export function CoverMatchProposal({
  operations,
  title,
  onChange,
}: {
  operations: Operations;
  title: string;
  onChange: (match: SteamTitle | null) => void;
}) {
  const t = useT();
  const [match, setMatch] = useState<SteamTitle | null>(null);
  // Whether the user picked a title or declined; then the proposal no longer follows the title.
  const [decided, setDecided] = useState(false);
  const [declined, setDeclined] = useState(false);
  // Held apart, so a new function from the parent is no reason to propose again.
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (decided) return;
    let active = true;
    operations.proposeCoverMatch(title).then(
      (proposal) => {
        if (!active) return;
        setMatch(proposal);
        onChangeRef.current(proposal);
      },
      // Without a proposal the Game keeps its icon; the user can still search.
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations, title, decided]);

  const choose = (chosen: SteamTitle | null) => {
    setDecided(true);
    setDeclined(chosen === null);
    setMatch(chosen);
    onChange(chosen);
  };

  return (
    <fieldset className="cover-match-proposal">
      <legend>{t('coverMatch.heading')}</legend>
      <CoverMatchView
        operations={operations}
        match={match}
        without={declined ? 'coverMatch.icon' : 'coverMatch.noMatch'}
        removeLabel="coverMatch.decline"
        preview
        onPick={choose}
        onRemove={() => choose(null)}
      />
    </fieldset>
  );
}

/**
 * The Cover Match of a Local Game on its detail page: the matched title,
 * which the user can change or remove, or the way to make one.
 * `onCoverChanged` hears each change, so every view loads the Cover again.
 */
export function CoverMatchChoice({
  operations,
  game,
  onCoverChanged,
}: {
  operations: Operations;
  game: Game;
  onCoverChanged: (game: Game) => void;
}) {
  const t = useT();
  const [match, setMatch] = useState<SteamTitle | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let active = true;
    // Not known, it shows as no match; a title can still be chosen.
    operations.coverMatch(game.id).then(
      (answer) => active && setMatch(answer),
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations, game.id]);

  const change = async (title: SteamTitle | null) => {
    setBusy(true);
    setFailed(false);
    try {
      await operations.setCoverMatch(game.id, title?.appId ?? null);
      setMatch(title);
      onCoverChanged(game);
    } catch {
      setFailed(true);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="cover-choice">
      <CoverMatchView
        operations={operations}
        match={match}
        removeLabel="coverMatch.remove"
        disabled={busy}
        onPick={change}
        onRemove={() => change(null)}
      />
      {failed && (
        <p className="note" role="alert">
          {t('coverMatch.changeFailed')}
        </p>
      )}
    </div>
  );
}
