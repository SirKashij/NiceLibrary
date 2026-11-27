# Covers are fetched at runtime and never redistributed

Game artwork belongs to the publishers, and Valve neither permits nor forbids third-party apps loading it from Steam's servers. We accept that grey area the way other launchers do: Covers are downloaded at runtime and cached only on the user's machine. No game image ever enters the repo or the installer, and README screenshots use placeholder Covers.

A Local Game gets its Cover through a chain, because no source without a server of our own always delivers the right picture:

1. Derive the title from the executable and folder name and look it up in Steam's app list; on a hit, use the Steam Cover. The user confirms or changes this Cover Match when adding the game.
2. Otherwise the executable's icon, until the user picks an image.

## Considered Options

- **SteamGridDB**: its Terms of Use allow only "personal, non-commercial use" and forbid "any automated use of the Site, the Service or the related systems", and no separate API terms permit an app like this one. Not built, not even as an opt-in with the user's own key ([research notes](../research/issue-56-going-public.md#2-steamgriddb-terms-and-the-opt-in-cover-source)).
- **IGDB**: needs a Twitch client secret per user, too heavy for setup.
- **RAWG**: requires an attribution link on every page that shows its data.
- **Scraping sites such as HowLongToBeat**: their terms do not allow it.

## Consequences

- A Local Game that is not on Steam starts with its executable's icon; the user can pick their own image.
- The Steam logo is never a prominent element of the interface (Valve branding guidelines).
