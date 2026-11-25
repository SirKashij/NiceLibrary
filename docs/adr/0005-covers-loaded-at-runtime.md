# Covers are fetched at runtime and never redistributed

Game artwork belongs to the publishers, and Valve neither permits nor forbids third-party apps loading it from Steam's servers. We accept that grey area the way other launchers do: Covers are downloaded at runtime and cached only on the user's machine. No game image ever enters the repo or the installer, and README screenshots use placeholder Covers.

A Local Game gets its Cover through a chain, because no source without a server of our own always delivers the right picture:

1. Derive the title from the executable and folder name and look it up in Steam's app list; on a hit, use the Steam Cover. The user confirms or changes this Cover Match when adding the game.
2. Otherwise SteamGridDB, if the user has stored their own key there.
3. Otherwise the executable's icon, until the user picks an image.

## Considered Options

- **IGDB**: needs a Twitch client secret per user, too heavy for setup.
- **RAWG**: requires an attribution link on every page that shows its data.
- **Scraping sites such as HowLongToBeat**: their terms do not allow it.

## Consequences

- Step 2 is opt-in and only ships once the SteamGridDB terms have been read by hand; they could not be retrieved automatically.
- The Steam logo is never a prominent element of the interface (Valve branding guidelines).
