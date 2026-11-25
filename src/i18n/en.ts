// English interface texts. This file defines the set of keys; every other language must cover all of them.
export const en = {
  'app.name': 'NiceLibrary',
  'nav.main': 'Main',
  'nav.library': 'Library',
  'nav.statistics': 'Statistics',
  'nav.settings': 'Settings',
  'library.empty': 'Your library is empty. Games from Steam and games you add yourself will show up here.',
  'library.loadFailed': 'The library could not be loaded.',
  'statistics.empty': 'Statistics appear once you have played.',
  'settings.empty': 'There is nothing to set yet.',
} as const;

export type MessageKey = keyof typeof en;
