import { useT, type MessageKey } from '../i18n';
import type { Status } from '../operations';

/** The text of each Status. */
export const STATUS_LABELS: Record<Status, MessageKey> = {
  unplayed: 'status.unplayed',
  playing: 'status.playing',
  completed: 'status.completed',
  dropped: 'status.dropped',
};

/** Every Status, in the order the interface offers them. */
export const STATUSES: Status[] = ['unplayed', 'playing', 'completed', 'dropped'];

/** A Game's Status as a small label, as the Library shows it. */
export function StatusBadge({ status }: { status: Status }) {
  const t = useT();
  return (
    <span className="status-badge" data-status={status}>
      {t(STATUS_LABELS[status])}
    </span>
  );
}

/** The four Statuses to choose from, with the Game's own pressed. */
export function StatusChoice({
  title,
  status,
  onChoose,
}: {
  /** The Game's title, which names the choice. */
  title: string;
  status: Status;
  onChoose: (status: Status) => void;
}) {
  const t = useT();
  return (
    <div className="segmented" role="group" aria-label={t('status.of', { title })}>
      {STATUSES.map((choice) => (
        <button
          key={choice}
          type="button"
          aria-pressed={choice === status}
          onClick={() => choice !== status && onChoose(choice)}
        >
          {t(STATUS_LABELS[choice])}
        </button>
      ))}
    </div>
  );
}
