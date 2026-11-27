import { useEffect, useState } from 'react';
import { useT, type MessageKey } from './i18n';
import type { Game, Operations } from './operations';

/**
 * The choice of a Local Game's Cover, on its detail page: an image of the
 * user's own, or back to the executable's icon once one is chosen.
 * `onCoverChanged` hears each change, so every view loads the Cover again.
 */
export function LocalGameCoverChoice({
  operations,
  game,
  onCoverChanged,
}: {
  operations: Operations;
  game: Game;
  onCoverChanged: (game: Game) => void;
}) {
  const t = useT();
  // Unknown until the core answers; the way back to the icon shows only once it is known.
  const [chosen, setChosen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<MessageKey>();

  useEffect(() => {
    let active = true;
    // Not known, the way back to the icon stays hidden; choosing an image still works.
    operations.coverImageChosen(game.id).then(
      (answer) => active && setChosen(answer),
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations, game.id]);

  const change = async (how: () => Promise<boolean | undefined>) => {
    setBusy(true);
    setProblem(undefined);
    try {
      const nowChosen = await how();
      if (nowChosen !== undefined) {
        setChosen(nowChosen);
        onCoverChanged(game);
      }
    } catch {
      setProblem('cover.changeFailed');
    } finally {
      setBusy(false);
    }
  };

  const chooseImage = () =>
    change(async () => {
      const { outcome } = await operations.chooseCoverImage(game.id);
      if (outcome === 'notAnImage') setProblem('cover.notAnImage');
      return outcome === 'chosen' ? true : undefined;
    });

  const useIcon = () =>
    change(async () => {
      await operations.removeCoverImage(game.id);
      return false;
    });

  return (
    <div className="cover-choice">
      <button type="button" className="button" disabled={busy} onClick={chooseImage}>
        {t('cover.chooseImage')}
      </button>
      {chosen && (
        <button type="button" className="button" disabled={busy} onClick={useIcon}>
          {t('cover.useIcon')}
        </button>
      )}
      {problem && (
        <p className="note" role="alert">
          {t(problem)}
        </p>
      )}
    </div>
  );
}
