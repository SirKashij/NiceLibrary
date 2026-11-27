# NiceLibrary

A Windows desktop app that gathers a person's games from Steam and from their own disk into one library, tracks how much and how far they have played, and launches the games. NiceLibrary is a working title.

## Language

### Library

**Library**:
The complete set of Games known to the app for one person, across all Sources.
_Avoid_: Collection (that is a different term), catalogue

**Game**:
One playable title in the Library. A Game belongs to exactly one Source.
_Avoid_: App, title, entry

**Source**:
The origin a Game is read from. There are two: the Steam Source and the Local Source.
_Avoid_: Launcher, platform, provider, store

**Steam Game**:
A Game that comes from the Steam Source, whether or not it is Installed.

**Local Game**:
A Game the person added by hand by pointing at its executable.
_Avoid_: Non-Steam game, custom game, manual game

**Installed**:
A Game whose files are present on this computer and can be launched right now. A Local Game is always Installed; a Steam Game may be owned but not Installed.

**Steam Account**:
The Steam identity whose Library is read. One installation of the app works with exactly one Steam Account.
_Avoid_: User, profile

### Tracking

**Session**:
One uninterrupted period during which a Game's process was running, regardless of whether its window had focus.
_Avoid_: Play session, run

**Playtime**:
The total time a Game has been played. For a Steam Game it is the figure Steam reports; for a Local Game it is the sum of its Sessions plus the Playtime Adjustment.
_Avoid_: Hours, time played

**Playtime Adjustment**:
An amount of time the person adds to or removes from a Local Game's Playtime by hand, for example to record time played before the app was installed.

**Last Played**:
The moment a Game's most recent Session ended, or "never".

**Achievement**:
A goal defined by a Steam Game that Steam marks as unlocked for the Steam Account. Local Games have no Achievements. The German interface calls it "Trophäe".
_Avoid_: Trophy (in English text), badge

**Secret Achievement**:
An Achievement whose name and description the app conceals until it is unlocked or the person chooses to reveal it.
_Avoid_: Hidden achievement (clashes with Hidden Game)

**Rarity**:
The share of all players worldwide who have unlocked an Achievement, in percent.

**Progress**:
How far a Game has been completed, in percent. For a Steam Game it is the share of unlocked Achievements; for a Local Game it is a Manual Progress value or nothing.
_Avoid_: Completion, story progress

**Manual Progress**:
A percentage the person sets by hand on a Local Game.

**Status**:
The person's own judgement of where they stand with a Game: Unplayed, Playing, Completed or Dropped. It is set by hand and is independent of Progress.
_Avoid_: State

### Organising

**Favorite**:
A Game the person has marked to find it quickly.

**Hidden Game**:
A Game the person has removed from the normal Library view without deleting it.
_Avoid_: Ignored, archived

**Collection**:
A named group of Games created by the person. A Game can be in several Collections.
_Avoid_: Category, tag, folder, list

**Detail Page**:
The page of one Game, opened from the Library: its large Cover, Source, whether it is Installed, its Status, Playtime, Last Played and its Launch. Going back returns to the Library as it was left.
_Avoid_: Game page, info screen, dialog

**Sort Order**:
The order the Library is shown in: recently played (by Last Played, the default), name or Playtime. Games without a value come last, Games with the same value by title. The chosen Sort Order survives a restart.
_Avoid_: Sorting, ranking

### Artwork

**Cover**:
The upright picture that represents a Game in the Library.
_Avoid_: Box art, poster, artwork, thumbnail

**Cover Match**:
The link between a Local Game and the Steam title whose Cover it borrows. The person confirms or changes it when adding the Game.

**Tint**:
The background and accent colour a Detail Page takes from the dominant hue of its Game's Cover, the hue most of the Cover's colourful pixels share. A Game without a Cover, a Cover without colour, or a Tint that would miss the contrast rules of the shown Theme leaves the page neutral.
_Avoid_: Theme (that is dark or light), accent colour (alone), palette

**Steam App List**:
Steam's list of the titles it offers, each with its app id and name, fetched with the Steam Web API key and kept on this computer as the basis for Cover Matches. It is fetched, not synced: it belongs to no Steam Account.
_Avoid_: App database, catalogue, Sync (for fetching it)

### Operation

**Sync**:
One round of fetching the Steam Account's Games, Playtime and Achievements from Steam.
_Avoid_: Refresh, update (update means a new app version)

**Steam Web API key**:
The person's own key for Steam's Web API, which the app needs to read the Steam Account's Games, Playtime and Achievements. It is personal and is kept only in the operating system's secret store.
_Avoid_: API key (alone), token, password

**Limited Mode**:
How the app runs when it cannot read the Steam Account's data from Steam: Installed Steam Games and launching work, Achievements are missing.
_Avoid_: Offline mode, basic mode

**Launch**:
Starting a Game from the app.
_Avoid_: Run, start, open

**Log**:
The files on this computer the app writes its errors and notable events to, for the person to attach to a bug report themselves. It never holds the Steam Web API key or a full SteamID, and the app never sends it.
_Avoid_: Crash report, telemetry, diagnostics

### Interface

**Theme**:
Whether the interface shows dark or light. Until the person chooses one in the settings, it follows the operating system's setting. The German interface calls it "Design".
_Avoid_: Skin, mode, appearance

**Tray**:
The app's icon in the notification area of the Windows taskbar. While the window is closed the app keeps running there, so tracking goes on, unless the person turned close-to-tray off; its menu opens the window or quits the app. The German interface calls it "Infobereich".
_Avoid_: System tray (in interface text), taskbar, background
