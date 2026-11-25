import type { MessageKey } from './en';

// German interface texts. The type makes a missing key a compile error.
export const de: Record<MessageKey, string> = {
  'app.name': 'NiceLibrary',
  'nav.main': 'Hauptnavigation',
  'nav.library': 'Bibliothek',
  'nav.statistics': 'Statistik',
  'nav.settings': 'Einstellungen',
  'library.empty': 'Deine Bibliothek ist leer. Spiele aus Steam und Spiele, die du selbst hinzufügst, erscheinen hier.',
  'library.loadFailed': 'Die Bibliothek konnte nicht geladen werden.',
  'statistics.empty': 'Statistiken erscheinen, sobald du gespielt hast.',
  'settings.empty': 'Noch gibt es nichts einzustellen.',
};
