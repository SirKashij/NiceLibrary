import { useT } from './i18n';
import type { Achievement } from './operations';

/**
 * The switch that reveals the locked Secret Achievements of a Game. Shown
 * only when the Game has Secret Achievements; off whenever the page opens.
 */
export function RevealSecrets({
  achievements,
  revealed,
  onChange,
}: {
  achievements: Achievement[];
  revealed: boolean;
  onChange: (revealed: boolean) => void;
}) {
  const t = useT();
  if (!achievements.some((achievement) => achievement.secret)) {
    return null;
  }
  return (
    <label className="checkbox-row">
      <input type="checkbox" role="switch" checked={revealed} onChange={(event) => onChange(event.target.checked)} />
      {t('achievements.revealSecrets')}
    </label>
  );
}

/**
 * `achievement` as the page shows it: a locked Secret Achievement that is not
 * revealed gets a neutral name and description and no icon, so that nothing
 * about it is spoiled; every other Achievement is shown as it is.
 */
export function useShown(revealed: boolean): (achievement: Achievement) => Achievement {
  const t = useT();
  return (achievement) =>
    achievement.secret && !achievement.unlocked && !revealed
      ? { ...achievement, name: t('achievements.secret'), description: t('achievements.secretHint'), icon: null }
      : achievement;
}
