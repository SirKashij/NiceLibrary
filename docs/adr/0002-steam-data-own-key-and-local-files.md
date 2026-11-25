# Steam data comes from the user's own Web API key plus local Steam files

Steam's Web API Terms of Use make a key personal and confidential, so an open-source app cannot ship one. Each user enters their own key; the app combines the Web API (owned games, playtime, achievements, rarity) with Steam's local files (what is installed, which accounts exist, fallback playtime). The key is kept in the operating system's secret store, never in the repo or the database.

## Considered Options

- **A shared key behind our own proxy**: would hide the key but needs a server, which contradicts ADR-0003.
- **Local files only**: no key needed, but no reliable achievements and nothing about games that are not installed.

## Consequences

- Without a key, or when Steam refuses the data, the app runs in Limited Mode.
- Steam Games are launched through the `steam://` protocol, never by starting their executable directly, so Steam keeps handling overlay, cloud saves, playtime and copy protection.
- The terms require a privacy notice, telling users which Steam data is stored, and no appearance of affiliation with Valve. README and the About dialog carry these texts and the Valve trademark notice.
- The Steam account is detected from the local `loginusers.vdf`; a machine can hold several accounts, so setup offers a choice. The key should belong to the chosen account.

## Findings from the throwaway check (2026-11-25, `prototype/steam-check`)

- With a public profile and public game details, the key owner gets everything: owned games with playtime and last-played time, achievements with unlock time, the schema with icons and the hidden flag, and global percentages.
- For an account whose game details are private, the API returned almost no games, no playtime, and refused achievements with "Profile is not public", even though the profile itself reported as public. Setup therefore has to explain the "game details" privacy setting.
- Not yet tested: whether a key owner can read their own account while its game details are private.
- `ISteamApps/GetAppList` is gone (404). `IStoreService/GetAppList` works with a key and is paged.
