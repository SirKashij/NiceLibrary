# The Export File format

An Export File holds everything in NiceLibrary that is the person's own, so it can serve as a backup or move to another computer. The app writes it from **Settings → Backup** (`export_data` in `crates/core/src/lib.rs`, format in `crates/core/src/export.rs`); an import reads it back (#52).

It is one UTF-8 JSON file, written in one step (a failed export leaves no half file). The app offers the name `NiceLibrary-<yyyy>-<mm>-<dd>.json`, with the date in UTC.

## What is not in it

- **The Steam Web API key.** It lives only in the Windows Credential Manager and never goes into a file.
- **Anything the app can fetch again:** Steam Games and Steam's figures (Playtime, Last Played, Installed), Achievements, the Steam App List, the date of the last Sync, and cached images (Steam Covers, Covers made from an executable's icon, Achievement icons). They come back from Steam and the disk.
- **The Log** and app-internal bookkeeping (such as whether the Tray notice was shown).

## Versions

| `version` | App version | Change |
| --- | --- | --- |
| 1 | 0.1.0 | First format. |

Rules for changing the format:

- A reader ignores fields it does not know.
- A new **optional** field may be added without a new version only if a reader that ignores it reads everything else correctly and loses nothing but that field.
- Any other change (a field removed, renamed, or given a new meaning or unit; an entry an older reader would misread) raises `version` by one and adds a row above.
- An import refuses a file whose `format` is not `nicelibrary-export`, whose `version` is newer than it knows, or that is not valid, and then changes nothing.

## Top level

```json
{
  "format": "nicelibrary-export",
  "version": 1,
  "appVersion": "0.1.0",
  "exportedAt": 1800000000,
  "settings": { },
  "localGames": [ ],
  "sessions": [ ],
  "statuses": [ ],
  "favorites": [ ],
  "hiddenGames": [ ],
  "collections": [ ]
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `format` | string | Always `nicelibrary-export`. |
| `version` | integer | The format version, see above. |
| `appVersion` | string | The version of the app that wrote the file. Informational. |
| `exportedAt` | integer | When the file was written, in seconds since 1970-01-01 UTC. |

All times are whole seconds since 1970-01-01 UTC. Lists may be empty, never missing.

### Game ids

Games are referred to by their Game id, unique across Sources:

- `steam:<appid>` for a Steam Game, for example `steam:620`. Stable across computers.
- `local:<n>` for a Local Game. The number is only valid **inside this file**: it names an entry of `localGames`. An import gives each Local Game a new number on the target computer and must rewrite every reference to it.

A Status, Favorite, Hidden Game or Collection membership may name a Steam Game that is currently not in the Library (for example because it is owned by another Steam Account or was not synced yet). It is kept as it is.

## `settings`

| Field | Type | Values |
| --- | --- | --- |
| `chosenSteamAccount` | string or null | The SteamID64 (as a string) of the Steam Account the person chose; `null` while none is chosen. |
| `theme` | string | `system`, `dark`, `light` |
| `language` | string | `system`, `de`, `en` |
| `libraryLayout` | string | `coverGrid`, `list` |
| `librarySort` | string | `recentlyPlayed`, `name`, `playtime`, `progress` |
| `closeToTray` | boolean | Whether closing the window keeps the app in the Tray. |
| `startWithWindows` | boolean | Whether the app starts with Windows (the autostart entry as it was when exporting). |

## `localGames`

One entry per Local Game, in the order they were added.

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | `local:<n>`, see Game ids. |
| `title` | string | The title. |
| `executable` | string | The absolute path of its executable, as chosen. It may not exist on another computer. |
| `launchArguments` | string | The Launch Arguments as typed; empty for none. |
| `workingDirectory` | string or null | The Working Directory; `null` for the executable's folder. |
| `manualProgress` | integer or null | The Manual Progress in percent, 0 to 100; `null` while none is set. |
| `playtimeAdjustmentMinutes` | integer or null | The Playtime Adjustment in minutes, negative when time was removed; `null` for none. |
| `coverImage` | object or null | The Cover image the person chose, `null` when they chose none. |

`coverImage` holds the image file itself:

| Field | Type | Meaning |
| --- | --- | --- |
| `mediaType` | string | `image/png` or `image/jpeg`. |
| `data` | string | The bytes of the image in standard Base64 (RFC 4648, with padding). |

Cover Matches (#37) do not exist yet. When they do, they are expected as an optional field of a Local Game.

## `sessions`

Every recorded Session, oldest first. A Game's Playtime is the sum of its Sessions plus its Playtime Adjustment, so the Sessions are kept one by one rather than as a total.

| Field | Type | Meaning |
| --- | --- | --- |
| `gameId` | string | The Game the Session belongs to. |
| `startedAt` | integer | When the Session began. |
| `endedAt` | integer | When it ended; never before `startedAt`. |

Version 1 of the app writes Sessions of Local Games only. Sessions of Steam Games (#39) are to be written into the same list with a `steam:<appid>` Game id and an extra field `steamAccount` (the SteamID64 as a string, or `null` when no Steam Account was known then). An import of version 1 must therefore accept entries with a `steam:` Game id and the optional `steamAccount` field.

## `statuses`

The Status the person set, by Game id in ascending order. A Game without an entry is Unplayed.

| Field | Type | Values |
| --- | --- | --- |
| `gameId` | string | The Game. |
| `status` | string | `unplayed`, `playing`, `completed`, `dropped` |

## `favorites` and `hiddenGames`

Lists of Game ids in ascending order: the Favorites, and the Hidden Games.

## `collections`

The Collections in the order they were created.

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | string | The name; names are unique regardless of case. |
| `gameIds` | array of strings | The Games in the Collection, in ascending order. |
