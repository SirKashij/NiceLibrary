import { useEffect, useState } from 'react';
import type { Operations } from '../operations';

/**
 * How many Sessions began or ended while the app runs. Each one changes a
 * Local Game's Playtime and Last Played, so views that show them read the
 * Library again when the number grows.
 */
export function useSessionChanges(operations: Operations): number {
  const [changes, setChanges] = useState(0);
  useEffect(() => operations.onSession(() => setChanges((n) => n + 1)), [operations]);
  return changes;
}
