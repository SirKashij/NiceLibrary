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

**Install Folder**:
The folder Steam installed an Installed Steam Game into, `steamapps/common/<folder>` in one of Steam's library folders. Any process whose executable lies in it is that Game running.

**Steam Account**:
The Steam identity whose Library is read: the one the person chose in the settings, by picking a Steam Login or by entering its SteamID, else the most recently used Steam Login. The app works with exactly one Steam Account at a time, and it need not be one of the Steam Logins. The German interface calls it "Konto für deine Bibliothek".
_Avoid_: User, profile

**SteamID**:
The 17-digit number by which Steam identifies an account. The person can name their Steam Account by its SteamID.
_Avoid_: Steam ID, user ID, account ID

**Steam Installation**:
The Steam program on this computer, whose own files the app reads: which Steam Games are Installed and where, its Steam Logins, and the Fallback Playtime.
_Avoid_: Steam client, Steam folder

**Steam Login**:
An account that has logged in to Steam on this computer and that Steam remembers, with the name it shows. One of them is the most recently used. The person picks their Steam Account from them, but the Steam Account need not be one of them. The interface calls them "Steam accounts on this computer", in German "Steam-Konten auf diesem Computer".
_Avoid_: Steam Account (for these), login (for the act of signing in), local account

### Tracking

**Session**:
One uninterrupted period during which a Game's process was running, regardless of whether its window had focus. The app notices the process however the Game was started: a Local Game by its executable, a Steam Game by its Install Folder. Every Session that ends starts a Sync. When the app is closed or crashes while the Game runs, the Session ends at the last moment the app saw the process. A Session whose process the app still sees is running. The German interface calls it "Spielsitzung".
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
How far a Game has been completed, in percent. For a Steam Game it is the share of unlocked Achievements, rounded down, so only a Game with every Achievement unlocked is complete at 100 %; a Steam Game without Achievements has no Progress rather than 0 %. For a Local Game it is a Manual Progress value or nothing.
_Avoid_: Completion, story progress

**Manual Progress**:
A percentage the person sets by hand on a Local Game.

**Status**:
The person's own judgement of where they stand with a Game: Unplayed, Playing, Completed or Dropped. It is set by hand and is independent of Progress.
_Avoid_: State

**Statistics**:
Figures over the Library that the core works out: total Playtime, unlocked Achievements of all and their share, the number of Games at 100 % Progress, the number of Games per Status, the Weekly Playtime and the five most played Games by Playtime. Hidden Games count in none of them.
_Avoid_: Stats, analytics

**Weekly Playtime**:
How long recorded Sessions ran in each of the last eight weeks, each week being the seven days up to a moment, so the latest ends now. A Session across the end of a week counts in both with its share. It counts only Sessions the app recorded, never Steam's Playtime figure or a Playtime Adjustment, since those do not say when a Game was played.
_Avoid_: Weekly stats, activity

### Organising

**Favorite**:
A Game the person has marked to find it quickly.

**Hidden Game**:
A Game the person has removed from the normal Library view without deleting it. It is left out of the search results, Favorites and Collections too, and stays hidden through Syncs and restarts until the person unhides it.
_Avoid_: Ignored, archived

**Collection**:
A named group of Games created by the person. A Game can be in several Collections.
_Avoid_: Category, tag, folder, list

**Detail Page**:
The page of one Game, opened from the Library: its large Cover, Source, whether it is Installed, its Status, Playtime, Last Played, Progress, its Launch and its Achievements. Going back returns to the Library as it was left.
_Avoid_: Game page, info screen, dialog

**Sort Order**:
The order the Library is shown in: recently played (by Last Played, the default), name or Playtime. Games without a value come last, Games with the same value by title. The chosen Sort Order survives a restart.
_Avoid_: Sorting, ranking

### Artwork

**Cover**:
The upright picture that represents a Game in the Library. A Steam Game's comes from Steam. A Local Game's is its Cover Image, else the Steam Cover of its Cover Match, else the icon of its executable on a neutral background, else there is none and the Library shows a neutral placeholder with the title.
_Avoid_: Box art, poster, artwork, thumbnail

**Cover Image**:
An image the person chose as a Local Game's Cover. The app keeps its own copy in its data folder, so moving or deleting the original changes nothing. Going back to the executable's icon forgets the copy.
_Avoid_: Custom cover, own cover

**Cover Match**:
The link between a Local Game and the Steam title whose Cover it borrows. When the Game is added, the app proposes the title of the Steam App List whose name is the Game's or begins with it; the person confirms it, picks another title or declines, and can change or remove it later on the Detail Page. Removing the Local Game forgets it.

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
How the app runs when it cannot read the Steam Account's data from Steam: Installed Steam Games and launching work, with the Fallback Playtime, and Achievements are missing. It has one of four reasons, which the app names: no key, key rejected, game details private, or Steam unreachable. It ends when a Sync succeeds.
_Avoid_: Offline mode, basic mode

**Fallback Playtime**:
The Playtime and Last Played that Steam keeps in its local files on this computer, shown for an Installed Steam Game until a Sync brings Steam's own figures.

**Launch**:
Starting a Game from the app.
_Avoid_: Run, start, open

**Launch Options**:
How a Local Game is launched: its Launch Arguments and its Working Directory, set by the person per Game.

**Launch Arguments**:
The arguments a Local Game's executable is started with. The person types them on one line, separated by spaces; a part in double quotes is one argument. Each reaches the executable as an argument of its own, never through a shell.
_Avoid_: Command line, parameters (in English text)

**Working Directory**:
The folder a Local Game's executable is started in. Without one, the executable's own folder.
_Avoid_: Start folder, current directory

**Export File**:
One file the person saves from the settings that holds everything in the app that is their own: Local Games with their Launch Options and chosen Cover images, Sessions, Playtime Adjustments, Manual Progress, Statuses, Favorites, Hidden Games, Collections and the settings. It is a backup, and the way to move to another computer by importing it there. It never holds the Steam Web API key or anything the app can fetch again from Steam. Its format is versioned and documented in docs/export-format.md. The German interface calls the area "Datensicherung".
_Avoid_: Backup file, dump, archive

**Import**:
Reading an Export File back, on the same or another computer. It replaces everything that is the person's own with what the file holds, after the app said so and the person confirmed; Steam Games and the Steam Web API key stay as they are. Each Local Game gets a new number. A file of a newer version or a damaged one is refused and changes nothing.
_Avoid_: Restore, merge, sync

**Install**:
Asking Steam, from the app, to install a Steam Game that is owned but not Installed. Steam shows its own install dialog and does the installing; the Game counts as Installed once Steam reports it so.
_Avoid_: Download, setup

**Log**:
The files on this computer the app writes its errors and notable events to, for the person to attach to a bug report themselves. It never holds the Steam Web API key or a full SteamID, and the app never sends it.
_Avoid_: Crash report, telemetry, diagnostics

**Unreadable Database**:
The reduced state the app starts in when its database cannot be opened, for one of three reasons: a newer version of the app wrote it, the file is damaged, or its folder cannot be written. The app then opens only its window, which names the reason and the database's full path and offers to show its folder or quit; the Library is not shown, and nothing runs that needs the database. The file is never changed, moved or deleted.
_Avoid_: Corrupt database, database error, safe mode

### Interface

**Theme**:
Whether the interface shows dark or light. Until the person chooses one in the settings, it follows the operating system's setting. The German interface calls it "Design".
_Avoid_: Skin, mode, appearance

**Library Layout**:
How the Library shows its Games: as a cover grid, one card with a large Cover per Game, or as a list, one row per Game with a small Cover, title, Status, Source, Playtime, Last Played, its Favorite star and its Launch. Both show the same Games in the same order. Until the person switches, it is the cover grid. The German interface calls it "Ansicht".
_Avoid_: View (the app's pages are views), mode, display

**Interface language**:
Whether the interface shows German or English, its texts as well as how it writes dates, numbers and durations. Until the person chooses one in the settings, it follows the system language, with English for anything but German. The settings name each language in itself ("Deutsch", "English"); the German interface calls the setting "Sprache".
_Avoid_: Locale, translation, i18n (in interface text)

**Tray**:
The app's icon in the notification area of the Windows taskbar. While the window is closed the app keeps running there, so tracking goes on, unless the person turned close-to-tray off; its menu opens the window or quits the app. The German interface calls it "Infobereich".
_Avoid_: System tray (in interface text), taskbar, background

**Start with Windows**:
The app starting by itself when the person signs in to Windows, hidden in the Tray so that tracking never depends on them remembering. It is off until they turn it on, and it is the current user's autostart entry itself, so it shows what Windows will really do. The German interface says "Mit Windows starten".
_Avoid_: Autostart (in interface text), launch at login, boot (Launch is starting a Game)
