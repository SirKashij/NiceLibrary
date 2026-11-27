import { useEffect, useId, useState } from 'react';
import { useFormat, useT } from './i18n';
import type { Achievement, Game, GameAchievements, Operations } from './operations';
import { RevealSecrets, useShown } from './SecretAchievements';

/**
 * The Achievements section of a Game's detail page: every Achievement with
 * its icon, name, description and unlock date, unlocked ones first, and how
 * many of all are unlocked; else why there are none. Reads them again after
 * each Sync.
 */
export function Achievements({ operations, game, syncs }: { operations: Operations; game: Game; syncs: number }) {
  const t = useT();
  const format = useFormat();
  const headingId = useId();
  const [answer, setAnswer] = useState<GameAchievements | 'loading' | 'failed'>('loading');
  const [revealed, setRevealed] = useState(false);
  const shown = useShown(revealed);

  useEffect(() => {
    let active = true;
    operations.achievements(game.id).then(
      (achievements) => active && setAnswer(achievements),
      // Achievements already shown stay until the next answer replaces them.
      () => active && setAnswer((shown) => (shown === 'loading' ? 'failed' : shown)),
    );
    return () => {
      active = false;
    };
  }, [operations, game.id, syncs]);

  return (
    <section className="achievements" aria-labelledby={headingId}>
      <div className="achievements-heading">
        <h2 id={headingId}>{t('achievements.title')}</h2>
        {typeof answer === 'object' && answer.state === 'listed' && (
          <span className="muted">
            {t('achievements.count', { unlocked: format.count(answer.unlocked), total: format.count(answer.total) })}
          </span>
        )}
      </div>
      {typeof answer === 'object' && answer.state === 'listed' && (
        <RevealSecrets achievements={answer.achievements} revealed={revealed} onChange={setRevealed} />
      )}
      {answer === 'failed' && <p role="alert">{t('achievements.loadFailed')}</p>}
      {typeof answer === 'object' &&
        (answer.state === 'listed' ? (
          <ul className="achievement-list" aria-label={t('achievements.list', { title: game.title })}>
            {answer.achievements.map((achievement) => (
              <AchievementItem key={achievement.id} achievement={shown(achievement)} iconUrl={(icon) => operations.achievementIconUrl(game, icon)} />
            ))}
          </ul>
        ) : (
          <p className="note muted">
            {answer.state === 'localGame'
              ? t('achievements.localGame')
              : answer.state === 'noAchievements'
                ? t('achievements.none', { title: game.title })
                : t('achievements.notFetchedYet')}
          </p>
        ))}
    </section>
  );
}

function AchievementItem({ achievement, iconUrl }: { achievement: Achievement; iconUrl: (icon: string) => string }) {
  const t = useT();
  const format = useFormat();
  const [iconFailed, setIconFailed] = useState(false);
  const status = !achievement.unlocked
    ? t('achievements.locked')
    : achievement.unlockedAt === null
      ? t('achievements.unlocked')
      : t('achievements.unlockedOn', { date: format.date(achievement.unlockedAt) });

  return (
    <li className="achievement" data-unlocked={achievement.unlocked}>
      <div className="achievement-icon">
        {achievement.icon !== null && !iconFailed && (
          // Decorative: the name beside it says what it is.
          <img src={iconUrl(achievement.icon)} alt="" loading="lazy" decoding="async" onError={() => setIconFailed(true)} />
        )}
      </div>
      <div className="achievement-text">
        <h3>{achievement.name}</h3>
        {achievement.description !== null && <p>{achievement.description}</p>}
        <p className="achievement-status">{status}</p>
        {achievement.rarity !== null && (
          <p className="achievement-rarity">{t('achievements.rarity', { percent: format.percent(achievement.rarity) })}</p>
        )}
      </div>
    </li>
  );
}
