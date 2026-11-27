import { useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import { CoverMatchProposal } from '../CoverMatch';
import type { AddLocalGame, Operations, SteamTitle } from '../operations';

/** A message about adding: `status` for what went well, `alert` for what did not. */
type Notice = { role: 'status' | 'alert'; message: MessageKey; title?: string };

/** The executable the user chose, the title to add it under and its Cover Match, while they confirm. */
type Proposal = { executable: string; title: string; match?: SteamTitle | null };

/** What an answer of the core other than `added` tells the user. */
function refusal(answer: Exclude<AddLocalGame, { outcome: 'added' }>): Notice {
  switch (answer.outcome) {
    case 'alreadyAdded':
      return { role: 'alert', message: 'localGame.alreadyAdded', title: answer.title };
    case 'noTitle':
      return { role: 'alert', message: 'localGame.noTitle' };
    case 'notFound':
      return { role: 'alert', message: 'localGame.notFound' };
  }
}

/**
 * Adding a Local Game: the user chooses its executable in the file dialog,
 * confirms or corrects the proposed title and Cover Match, and the Game
 * joins the Library with them.
 * `onAdded` hears each Game the core added.
 */
export function AddLocalGameButton({ operations, onAdded }: { operations: Operations; onAdded: () => void }) {
  const t = useT();
  const [choosing, setChoosing] = useState(false);
  const [adding, setAdding] = useState(false);
  const [proposal, setProposal] = useState<Proposal>();
  const [notice, setNotice] = useState<Notice>();

  const choose = () => {
    if (choosing) return;
    setChoosing(true);
    setNotice(undefined);
    operations
      .chooseLocalGameExecutable()
      .then(
        (choice) => {
          if (choice.outcome === 'chosen') setProposal({ executable: choice.executable, title: choice.title });
          else if (choice.outcome === 'alreadyAdded')
            setNotice({ role: 'alert', message: 'localGame.alreadyAdded', title: choice.title });
        },
        () => setNotice({ role: 'alert', message: 'localGame.dialogFailed' }),
      )
      .finally(() => setChoosing(false));
  };

  const add = (chosen: Proposal) => {
    if (adding) return;
    setAdding(true);
    setNotice(undefined);
    operations
      .addLocalGame(chosen.executable, chosen.title)
      .then(
        async (answer) => {
          if (answer.outcome !== 'added') return setNotice(refusal(answer));
          setProposal(undefined);
          let notice: Notice = { role: 'status', message: 'localGame.added', title: answer.game.title };
          if (chosen.match) {
            // Saved before the Library loads the new Game's Cover.
            await operations.setCoverMatch(answer.game.id, chosen.match.appId).catch(() => {
              notice = { role: 'alert', message: 'coverMatch.notSaved', title: answer.game.title };
            });
          }
          setNotice(notice);
          onAdded();
        },
        () => setNotice({ role: 'alert', message: 'localGame.addFailed' }),
      )
      .finally(() => setAdding(false));
  };

  const cancel = () => {
    setProposal(undefined);
    setNotice(undefined);
  };

  return (
    <div className="add-local-game">
      {!proposal && (
        <button type="button" className="button" disabled={choosing} onClick={choose}>
          {t('localGame.add')}
        </button>
      )}
      {choosing && (
        <p className="note" role="status">
          {t('localGame.choosing')}
        </p>
      )}
      {proposal && (
        <form
          className="local-game-form"
          aria-label={t('localGame.proposal')}
          onSubmit={(event) => {
            event.preventDefault();
            add(proposal);
          }}
        >
          <label htmlFor="local-game-title">{t('localGame.title')}</label>
          <div className="field-row">
            <input
              id="local-game-title"
              type="text"
              autoComplete="off"
              aria-describedby="local-game-title-hint"
              value={proposal.title}
              onChange={(event) => setProposal({ ...proposal, title: event.target.value })}
            />
            <button type="submit" className="button" disabled={adding}>
              {t('localGame.confirm')}
            </button>
            <button type="button" className="button" onClick={cancel}>
              {t('localGame.cancel')}
            </button>
          </div>
          <p id="local-game-title-hint" className="muted">
            {t('localGame.titleHint')}
          </p>
          <div className="settings-row">
            <span>{t('localGame.executable')}</span>
            <span className="muted local-game-executable">{proposal.executable}</span>
          </div>
          <CoverMatchProposal
            operations={operations}
            title={proposal.title}
            onChange={(match) => setProposal((current) => current && { ...current, match })}
          />
        </form>
      )}
      {notice && (
        <p className="note" role={notice.role}>
          {t(notice.message, notice.title === undefined ? undefined : { title: notice.title })}
        </p>
      )}
    </div>
  );
}
