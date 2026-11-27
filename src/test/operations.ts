import type { Operations } from '../operations';

/**
 * Fake operations for interface tests that do nothing: an empty Library, no
 * Steam, no key, every dialog cancelled. A test spreads them and overrides
 * only the operations it cares about:
 *
 *     const operations: Operations = { ...baseOperations(), library: async () => games };
 *
 * A new operation gets its stand-in here, so the tests that do not use it stay
 * as they are.
 */
export function baseOperations(): Operations {
  return {
    database: async () => ({ state: 'open' }),
    openDatabaseFolder: async () => {},
    quit: async () => {},
    library: async () => [],
    readInstalledGames: async () => [],
    steamInstallation: async () => ({ state: 'notFound' }),
    steamAccount: async () => null,
    chooseSteamAccount: async () => ({ outcome: 'chosen' }),
    coverUrl: (game) => `cover://localhost/${game.id}`,
    launch: async () => ({ outcome: 'launched' }),
    steamWebApiKeyStored: async () => false,
    storeSteamWebApiKey: async () => ({ outcome: 'stored' }),
    removeSteamWebApiKey: async () => {},
    openSteamWebApiKeyPage: async () => {},
    sync: async () => ({ outcome: 'noKey' }),
    lastSync: async () => null,
    onAutomaticSync: () => () => {},
    chooseLocalGameExecutable: async () => ({ outcome: 'cancelled' }),
    addLocalGame: async () => ({ outcome: 'noTitle' }),
    theme: async () => 'system',
    chooseTheme: async () => {},
    collections: async () => [],
    createCollection: async () => ({ outcome: 'named', id: 1 }),
    renameCollection: async (id) => ({ outcome: 'named', id }),
    deleteCollection: async () => {},
    steamAppList: async () => ({ titles: 0, fetchedAt: null, unfinished: false }),
    fetchSteamAppList: async () => ({ outcome: 'noKey' }),
    onAutomaticSteamAppListFetch: () => () => {},
    filterLibrary: async () => [],
    game: async () => null,
    gameCollections: async () => [],
    addGameToCollection: async () => {},
    removeGameFromCollection: async () => {},
    libraryLayout: async () => 'coverGrid',
    chooseLibraryLayout: async () => {},
    closeToTray: async () => true,
    setCloseToTray: async () => {},
    closeWindow: async () => {},
    onTrayNotice: () => () => {},
    showTrayMenu: async () => {},
    librarySort: async () => 'name',
    chooseLibrarySort: async () => {},
    language: async () => 'system',
    chooseLanguage: async () => {},
    setManualProgress: async () => ({ outcome: 'notALocalGame' }),
    chooseCoverImage: async () => ({ outcome: 'cancelled' }),
    coverImageChosen: async () => false,
    removeCoverImage: async () => {},
    setStatus: async () => {
      throw new Error('this test sets no Status');
    },
    openLogFolder: async () => {},
    limitedMode: async () => ({ outcome: 'off' }),
    openSteamPrivacySettings: async () => {},
    startsWithWindows: async () => false,
    setStartWithWindows: async () => {},
    onSession: () => () => {},
    playtimeAdjustment: async () => null,
    setPlaytimeAdjustment: async () => {
      throw new Error('this test adjusts no Playtime');
    },
    sessions: async () => [],
    achievements: async () => ({ state: 'notFetchedYet' }),
    achievementIconUrl: (game, icon) => `cover://localhost/${game.id}/achievements/${icon}`,
    launchOptions: async () => null,
    setLaunchOptions: async () => {},
    hiddenGames: async () => [],
    hideGame: async () => {},
    unhideGame: async () => {},
    localGame: async () => null,
    chooseExecutableFor: async () => ({ outcome: 'cancelled' }),
    editLocalGame: async () => ({ outcome: 'noTitle' }),
    removeLocalGame: async () => {},
    statistics: async () => {
      throw new Error('this test shows no Statistics');
    },
    markFavorite: async () => {},
    favoriteCount: async () => 0,
    about: async () => ({ version: '0.1.0', license: 'MIT' }),
    openAboutPage: async () => {},
    exportData: async () => ({ outcome: 'cancelled' }),
    coverHue: async () => null,
    install: async () => ({ outcome: 'started' }),
    suggestExecutablesInFolder: async () => ({ outcome: 'cancelled' }),
    findSteamTitles: async () => [],
    proposeCoverMatch: async () => null,
    coverMatch: async () => null,
    setCoverMatch: async () => {},
    importData: async () => ({ outcome: 'cancelled' }),
  };
}
