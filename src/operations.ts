// The interface talks to the application core only through these operations.
// In the app they are Tauri commands; tests hand the interface a fake.
import { invoke } from '@tauri-apps/api/core';

export type Source = 'steam' | 'local';

/** One playable title in the Library. Mirrors `Game` in crates/core. */
export interface Game {
  id: string;
  title: string;
  source: Source;
  installed: boolean;
}

export interface Operations {
  /** All Games in the Library. */
  library(): Promise<Game[]>;
}

export const tauriOperations: Operations = {
  library: () => invoke<Game[]>('library'),
};
