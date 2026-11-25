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
  'settings.steam': 'Steam',
  'steam.found': 'Steam-Installation gefunden',
  'steam.notFound': 'Steam wurde auf diesem Computer nicht gefunden.',
  'steam.loadFailed': 'Steam konnte nicht geprüft werden.',
  'steam.accounts': 'Steam-Konten auf diesem Computer',
  'steam.noAccounts': 'Auf diesem Computer hat sich noch kein Steam-Konto angemeldet.',
  'steam.mostRecent': 'Zuletzt verwendet',
};
