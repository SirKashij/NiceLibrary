# Research: before going public (issue #56)

Research notes for [issue #56](https://github.com/SirKashij/NiceLibrary/issues/56), gathered on 2026-11-27. Every acceptance criterion of the issue is a task only the maintainer can do. These notes collect what the primary sources say, where the repo stands, and what is still open. They are not legal advice.

Markers used below: **Verified** = read in the primary source or the code. **Unverified** = the primary source could not be retrieved, or the statement is an inference; it is marked as such where it appears.

## Decisions (2026-11-27)

- **Name:** the project stays NiceLibrary. No rename of repo, identifier or data folder.
- **SteamGridDB:** not built, not even as an opt-in. Recorded in ADR-0005.
- **History:** the sample user name in the profile-path examples is replaced in a new commit. The history is not rewritten.
- **SignPath:** the "Code signing policy" section is in the README (English and German) ahead of the application.
- **Steam, private game details:** the test on the maintainer's own account is still open.

## Summary

- **The repo is already public.** `gh api repos/SirKashij/NiceLibrary` reports `"visibility": "public"`. ADR-0006 says "The repo stays private until v0.1.0". So the history audit (criterion 4) is no longer a gate before publication. It is now a check of what is already exposed, including 27 `refs/pull/*` refs on GitHub that a force push cannot remove.
- **History audit:** every author and committer identity on all branches and all PR refs is either the allowed `SirKashij <258232409+SirKashij@users.noreply.github.com>` (429 of 456 identity slots) or GitHub's own web-flow committer `GitHub <noreply@github.com>` on 27 merge commits made in the web UI. There are no non-allowed personal identities. One content finding: the maintainer's Windows user name is used as the sample user name in profile-path examples in `crates/core/src/log.rs:200` and `crates/core/tests/log.rs:344-353`, introduced in commit `39c1f81` and already pushed. No real SteamID of the local Steam accounts and no API key appear anywhere in the history.
- **Rename:** the identifier `io.github.sirkashij.nicelibrary` decides `%APPDATA%\<id>`, `%LOCALAPPDATA%\<id>` (cache, WebView2 data), the single-instance mutex, the Credential Manager prefix and what the uninstaller deletes. `productName` decides the NSIS uninstall registry key, the install folder and the Run-key value. Changing either one after a release strands the users' data or produces a second installed app, so both must be final before v0.1.0. Side finding: with no `bundle.publisher` set, Tauri uses the second part of the identifier, so Windows currently lists the publisher as **"github"**.
- **SteamGridDB:** there are no separate API terms. The site-wide Terms of Use (effective 2019-07-31) allow only "personal, non-commercial use" and forbid "any automated use of the Site, the Service or the related systems". Yet SteamGridDB publishes an API with per-user keys and lists third-party launchers that use it. The opt-in Cover source with the user's own key fits the way the API is offered, but the terms' wording does not cover it explicitly. Rate limits are not documented. A written confirmation from SteamGridDB would resolve this.
- **Steam, private game details:** Valve's docs say only that `GetOwnedGames` returns games "if their owned games/game details are visible to you". Whether "you" includes the key owner looking at their own account is undocumented and can only be settled empirically (procedure below). Steam OAuth access tokens need a Client ID that Valve issues to partners, so they are not an option for this app.
- **SignPath Foundation:** the conditions match ADR-0006 (already released, OSI license, CI builds on GitHub-hosted runners, manual approval, MFA). They add requirements the repo does not meet yet: a "Code signing policy" section on the home page with a fixed attribution sentence, team roles and a privacy statement, and a product name in the binaries that matches the project name. That is one more reason to finish the rename first.
- **Trademarks:** DPMAregister has **no similarity search** (DPMA states this itself) and covers EU and IR marks only without warranty. DPMA points to TMview, eSearch plus and Madrid Monitor for a complete search and recommends searching for unregistered rights too. The relevant Nice classes are 9 first (downloadable software, including game software), then 42, and possibly 41.

---

## 1. Final name, trademark check, rename

### What the sources say

**DPMA on trademark searches** ([dpma.de, "Marken recherchieren"](https://www.dpma.de/recherchieren/schutzrechte-recherchieren/marken-recherchieren), published 2025-12-10). Verified:

- Purpose: "Vor Anmeldung einer Marke können Sie durch eine Recherche möglichst weitgehend feststellen, ob Ihre geplante Marke keine älteren Rechte verletzt. Sonst kann ein Dritter gegen Ihre Marke Widerspruch beim DPMA erheben … Auch kann gegen Ihre Marke mit Abmahnung oder mit Klagen vor Zivilgerichten vorgegangen werden." The same risk applies to using a name without filing it, since use alone can infringe (§ 14 MarkenG, below).
- The office does not check conflicts itself: "Bitte beachten Sie, dass wir im Anmeldeverfahren nicht überprüfen, ob bereits ähnliche oder identische Marken eingetragen sind."
- **No similarity search in DPMAregister:** "… dass Sie in DPMAregister zwar Marken mit übereinstimmenden Elementen ihrer geplanten Marke finden können, eine Ähnlichkeitssuche aber nicht möglich ist. Ein Widerspruch … kann aber auch aus einer ähnlichen Marke erfolgen. Komplexe Ähnlichkeitsrecherchen werden von diversen kommerziellen Recherchedienstleistern angeboten."
- DPMAregister also contains EU marks (EM) and international registrations (IR) with effect in Germany, "Aber: Das DPMA übernimmt für diese Daten keine Gewähr … Für eine vollständige Recherche … verwenden Sie bitte die Datenbanken des … EUIPO und der … WIPO". Named tools: eSearch plus (EU marks), Madrid Monitor (IR marks), **TMview** ("Suchmaschine des EUIPO zu Marken und Markenanmeldungen aller teilnehmenden Markenämter") and the WIPO Global Brand Database.
- Unregistered rights count as well: "… nicht nur ältere Registermarken verletzen, sondern auch andere ältere Kennzeichen, wie Benutzungsmarken, geschäftliche Bezeichnungen und sonstige Rechte … Suchen Sie daher auch in Internet-Suchmaschinen, … Handelsregistern, Titelschutzanzeigen und/oder Branchen- und sonstigen Produktverzeichnissen nach identischen oder ähnlichen Zeichen."
- Free help is available: the DPMA database hotline for search questions (+49 89 2195-3435) and an "online-unterstützte Recherche" with DPMA staff.

**DPMAregister search modes** ([Hilfetext DPMAregister, PDF, V 1695/10.25](https://register.dpma.de/prod/de/register_hilfe.pdf), section 3 "Marken"). Verified:

- 3.2 **Basisrecherche**: a simple form with the most-used fields, including Nice class.
- 3.3 **Erweiterte Recherche**: several fields combined with UND/ODER/NICHT, with brackets.
- 3.4 **Expertenrecherche**: a query language. The help's examples show the wildcards: `MARKE=fili?` (starts with), `MARKE=?oase` (ends with), `MARKE=?brain?` (contains), `MARKE=tele!om` (one character: Telekom/Telecom), `MARKE=köl##sch` and `MARKE=para#i#s#` (optional characters), `MARKE=!#antas!#` (Fantasie/Phantasie/Fantasy). Spelling variants such as `e-mail`/`email`/`e mail` are matched automatically. **Use the wildcards by hand to approximate a similarity search**: spelling variants, dropped or added letters, sound-alikes, the name split into parts.

**TMview.** **Unverified**: `www.tmdn.org` could not be reached from this environment, so its help page ([TMview help](https://www.tmdn.org/static/tmdsview/tmview/helpFiles/en/help.html)) was not read. Search-engine snippets of that page say: a Nice class cannot be the only search criterion; choosing a territory selects its offices automatically; choosing "European Union" includes the national EU offices, EUIPO and WIPO registrations designating the EU or its member states. Third-party guides mention a "fuzzy" name search mode. Check the current search modes (exact / contains / fuzzy) in the tool itself. Related EUIPO tools: [TMclass](https://euipo.europa.eu/ec2/) (goods and services terms per class) and the [Similarity tool](https://euipo.europa.eu/sim/search) for goods and services. EUIPO states its results are not legally binding.

**When a name conflicts** ([§ 14 Abs. 2 MarkenG](https://www.gesetze-im-internet.de/markeng/__14.html)). Verified. Use of a sign is prohibited when (1) it is identical to the mark for identical goods or services; (2) it is identical or similar, for identical or similar goods or services, and "die Gefahr einer Verwechslung besteht"; (3) the mark is "im Inland bekannt" and the use exploits or harms its distinctiveness or reputation, for any goods or services. In practice: an identical name in class 9 or 42 is the clearest risk. A similar name (sound, spelling, meaning) in a related class is a risk. A famous mark is a risk in any class. [§ 5 MarkenG](https://www.gesetze-im-internet.de/markeng/__5.html) also protects *Werktitel* (work titles) without registration. Whether software titles count is not stated in the text (it says "sonstigen vergleichbaren Werken"). **Unverified**: German case law generally treats software titles as Werktitel; confirm with a professional if it matters.

**Nice classes for this app.** Partly verified through the [WIPO Nice Classification (NCL)](https://nclpub.wipo.int/enfr/?basic_numbers=show&class_number=9&explanatory_notes=show&lang=en&menulang=en&mode=flat&pagination=no) and the [EUIPO classification guidelines](https://guidelines.euipo.europa.eu/binary/2214311/2000140000) (the page text was not read in full; the statements come from search excerpts of these primary pages):

- **Class 9**: downloadable software. NCL lists "computer game software, downloadable" (basic no. 090829). The EUIPO guidelines say downloadable goods belong to class 9. A downloadable game launcher or library app is squarely class 9. **Most important class.**
- **Class 42**: design and development of software, SaaS, "providing temporary use of online non-downloadable" software. Relevant because marks for software companies usually cover 9 and 42 together, and 9 and 42 are often found similar.
- **Class 41**: entertainment, "on-line gaming services". This is the gaming side. Worth checking because the app is gaming-related.
- Class 28 (games apparatus) and 35 (retail, advertising) are lower priority. Check them only for a strong hit.

**Renaming the GitHub repo** ([GitHub Docs: Renaming a repository](https://docs.github.com/en/repositories/creating-and-managing-repositories/renaming-a-repository)). Verified:

- "all existing information, with the exception of project site URLs, is automatically redirected to the new name", including issues, wikis, stars and followers. Git operations against the old URL keep working, but "we strongly recommend updating any existing local clones" with `git remote set-url origin NEW_URL`.
- Not redirected: GitHub Pages project-site URLs, and "GitHub will not redirect calls to an action hosted by a renamed repository" (not relevant here, the repo hosts no action).
- "do not reuse the original name of the renamed repository … redirects to the renamed repository will no longer work."
- **Unverified inference:** links baked into a shipped binary (the About dialog's project and license URLs) keep working through the redirect only as long as nobody creates a repo under the old name.

**Tauri identifier and what depends on it.** Verified in the [config reference](https://v2.tauri.app/reference/config/) and in the source of the versions in `Cargo.lock` (tauri 2.12.7, tauri-utils 2.10.4, tauri-plugin-single-instance 2.5.5, CLI 2.12.6):

- `identifier`: "reverse domain name notation", "must be unique across applications since it is used in system configurations like the bundle ID and path to the webview data directory", "must contain only alphanumeric characters (A-Z, a-z, and 0-9), hyphens (-), and periods (.)".
- `app_data_dir()` = `dirs::data_dir()/<identifier>` = `%APPDATA%\<identifier>`. `app_local_data_dir()` = `%LOCALAPPDATA%\<identifier>`: "On Windows and Linux this is also the default data directory of the webviews" (WebView2 user data). `app_cache_dir()` = `dirs::cache_dir()/<identifier>` = `%LOCALAPPDATA%\<identifier>` on Windows (tauri `src/path/desktop.rs`). All can be redirected with `app > appDirectoriesOverride`. That would be a way to keep an old folder after a rename, but it adds complexity that is not needed before the first release.
- Single instance: the plugin derives its Windows mutex name from `app.config().identifier` (`tauri-plugin-single-instance` `src/platform_impl/windows.rs:81`). Two builds with different identifiers do not see each other.
- NSIS installer template ([installer.nsi at tauri-cli-v2.12.6](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.6/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi)):
  - `UNINSTKEY = Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}`. The installed-app entry, and with it upgrade detection and "is a previous version installed", is keyed by **productName**, not by the identifier. NSIS has no WiX-style upgrade code. Rename `productName` after a release and the next installer will not find the old installation: both versions stay installed.
  - The install folder for a per-user install is `$LOCALAPPDATA\${PRODUCTNAME}`.
  - The uninstaller deletes the Run value named `${PRODUCTNAME}`. With "Delete the application data" ticked it deletes `$APPDATA\${BUNDLEID}` and `$LOCALAPPDATA\${BUNDLEID}`, plus `Software\${MANUFACTURER}\${PRODUCTNAME}`.
  - `MANUFACTURER` = `bundle.publisher`, else "the second element in the identifier string" (tauri-utils `config.rs` doc comment; `nsis/mod.rs`: `bundle_id.split('.').nth(1)`). With `io.github.sirkashij.nicelibrary` and no `publisher`, the publisher shown in Windows' installed apps is **`github`**, and the registry key is `HKCU\Software\github\NiceLibrary`. Setting `bundle.publisher` explicitly fixes this.
- Updates: the working tree currently holds uncommitted work on an app-own update check (`crates/core/src/updates.rs`, `src-tauri/updater-key.pub`; no `tauri-plugin-updater` dependency). Per the uncommitted `CONTRIBUTING.md:104`, it fetches `https://github.com/SirKashij/NiceLibrary/releases/latest/download/latest.json` and downloads `NiceLibrary-{version}-setup.exe` (`updates.rs:197`). Running a newer NSIS installer runs the old uninstaller in update mode (`$UpdateMode = 1`, see `src-tauri/windows/hooks.nsh:8`). So an update after a rename runs into the same productName and identifier problems, and the URL compiled into v0.1.0 must survive any later rename.

### Current state in the repo

Name candidates: the glossary calls "NiceLibrary" a working title (`GLOSSARY.md:3`), and so does issue #1 ("NiceLibrary (working title)").

Rename checklist. Line numbers refer to the working tree on 2026-11-27 (`main` at `a49afd6` plus uncommitted work in progress on the update check, which touches README, CONTRIBUTING, `release.yml` and the i18n files), so they will drift. Re-run the `git grep` below before renaming.

Identity that users' machines depend on (must be final before v0.1.0):

- `src-tauri/tauri.conf.json:3` `productName` (NSIS uninstall key, install folder, Run value, shortcuts)
- `src-tauri/tauri.conf.json:5` `identifier` (data, cache and WebView2 folders, mutex, uninstall cleanup)
- `src-tauri/tauri.conf.json:17` window title
- `src-tauri/src/ports/secrets_windows.rs:16` `APP_PREFIX = "io.github.sirkashij.nicelibrary/"` (Credential Manager target)
- `src-tauri/windows/hooks.nsh:15` `StartupApproved\Run` value `"NiceLibrary"`; `hooks.nsh:21` `cmdkey /delete:io.github.sirkashij.nicelibrary/steam-web-api-key` (unit tests in `autostart_windows.rs` and `secrets_windows.rs` check these against the code)
- `src-tauri/src/ports/autostart_windows.rs:20` `NAME = "NiceLibrary"` (Run value; must equal productName, because Tauri's uninstaller deletes the value named after productName)
- `src-tauri/src/lib.rs:475` `DATABASE = "nicelibrary.db"`; `crates/core/src/log.rs:19,125` `nicelibrary.log`, `nicelibrary.N.log`
- `crates/core/src/export.rs:28` `FORMAT = "nicelibrary-export"` and `export.rs:259-262` file name `NiceLibrary-<date>.json` (the export format is a public contract, `docs/export-format.md`)
- Not set yet: `bundle.publisher` (currently resolves to "github", see above)

Binary and package names:

- `src-tauri/Cargo.toml:2` package `nicelibrary` (decides `nicelibrary.exe`), `:22` lib `nicelibrary_lib`; `crates/core/Cargo.toml` package `nicelibrary-core` (used in every `use nicelibrary_core` and in `.cargo/config.toml:3`)
- `package.json:2` `"name": "nicelibrary"`
- `.github/workflows/ci.yml:129,135` `NICELIBRARY_APP …/nicelibrary.exe`; `release.yml:15,205` (comment and release title)
- `e2e/app.ts:129-138`, `e2e/windows.ps1:92`, `e2e/windows.ts:45`, `e2e/smoke.e2e.ts:36`, `scripts/collect-e2e-diagnostics.ps1:7,15,23` (`webview-exe-name=nicelibrary`)
- `crates/core/src/updates.rs:197` (uncommitted, `NiceLibrary-{version}-setup.exe`) and the update URL documented in `CONTRIBUTING.md:104` (uncommitted)

URLs and texts:

- `crates/core/src/about.rs:30-31` project and license URLs (compiled into the binary; tests in `crates/core/tests/about.rs:55,65,77`)
- `src/i18n/en.ts` and `src/i18n/de.ts`: 29 occurrences each, starting with `app.name` (`en.ts:3`, `de.ts:5`); `index.html:6` `<title>`
- `README.md` (release URL at lines 23 and 101, installer file name at 28 and 106, and more), `CONTRIBUTING.md:1,7,32,54,56,81,85,89,104`, `SECURITY.md:8,16,20`, `CODE_OF_CONDUCT.md:3,22`, `LICENSE:3`, `GLOSSARY.md:1,3`, `CLAUDE.md:1,15`, `docs/export-format.md`, `.github/ISSUE_TEMPLATE/*.yml`, `deny.toml:1`
- Tests that name the app: `crates/core/tests/*.rs`, `src-tauri/tests/contract.rs` (also uses the test registry and credential names `NiceLibraryContractTests`), `src/*.test.tsx`

A repo-wide `git grep -n -i "nicelibrary\|sirkashij"` is the full list. Excluding crate-name imports leaves the items above.

### Open questions / must be verified by the maintainer

1. Pick the final name. Then search in TMview (all EU territories plus WIPO, classes 9, 42 and 41, exact, contains and fuzzy modes, status "registered" and "filed"), in DPMAregister (Expertenrecherche with wildcard variants, classes 9, 42 and 41), and on the web, GitHub, Steam, the Microsoft Store and the Handelsregister for unregistered uses. Write down what was searched and the date. For a hit in class 9 or 42 that is identical or close, choose another name or ask a professional. DPMA's hotline helps with the search technique, not with the legal assessment.
2. Decide the new identifier. Reverse-DNS under a domain you control; `io.github.<user>.<app>` is fine but ties the ID to the GitHub user name. Decide whether to set `bundle.publisher` (recommended; otherwise Windows shows "github").
3. Rename in this order: GitHub repo, `git remote set-url`, then every item of the checklist in one PR. Afterwards uninstall any locally installed dev build and delete the old `%APPDATA%`/`%LOCALAPPDATA%` folders and the old Credential Manager entry by hand, since nothing migrates them.
4. **Unverified:** whether GitHub's rename redirect also covers `releases/latest/download/latest.json` for unauthenticated HTTP clients. The docs list issues, wikis, stars and followers explicitly, and git operations. Rename before v0.1.0, so no shipped binary carries the old URL.

---

## 2. SteamGridDB terms and the opt-in Cover source

### What the sources say

The terms page is rendered by JavaScript, which is probably why it "could not be retrieved automatically" (ADR-0005). The text below comes from the site's own bundle for `/terms` (`https://www.steamgriddb.com/static/assets/ed6b9cabbbcb8e2128d3.js`, the chunk behind [steamgriddb.com/terms](https://www.steamgriddb.com/terms)). Verified, quoted verbatim:

- Title and date: "SteamGridDB, LLC Terms of Use (Effective as of 2019-07-31)". "Site" covers the website "or through our desktop application"; "Service" means the services "in which users can search for, download, and upload images related to video games".
- **USE RESTRICTIONS**, "You agree that you will not under any circumstances:"
  - "access the Service for any reason other than your personal, non-commercial use solely as permitted by the normal functionality of the Service,"
  - "use the Site or the Service for the solicitation of business in the course of trade or in connection with a commercial enterprise;"
  - "distribute any part or parts of the Site or the Service without our explicit written permission …"
  - "make any automated use of the Site, the Service or the related systems, or take any action that we deem to impose or to potentially impose an unreasonable or disproportionately large load on our servers or network infrastructure;"
  - "bypass any robot exclusion headers or other measures we take to restrict access to the Service, or use any software, technology, or device to scrape, spider, or crawl the Service or harvest or manipulate data;"
- **User content and image rights:** uploaders "retain all ownership rights" and grant "to each user of the Service, a worldwide, non-exclusive, royalty-free license to access your User Content through the Service, and to use, reproduce, distribute, prepare derivative works of, display and perform such Content to the extent permitted by the Service and under these Terms of Use". Uploaders must not submit copyrighted content "unless you are the owner of such rights or have the appropriate permission". The artwork is often derived from publishers' art, so the same grey area as ADR-0005 applies.
- **No attribution clause** and **no rate limit** appear in the terms.
- "We can amend these Terms of Use at any time … Your continued use of the Site or the Service signifies your agreement to our revisions". Governing law: Washington State, USA.

API documentation ([steamgriddb.com/api/v2](https://www.steamgriddb.com/api/v2), OpenAPI file `https://www.steamgriddb.com/static/openapi.yml`, version 2.10.0). Verified:

- Authentication: "Generate an API key here" (link to `/profile/preferences`, so a SteamGridDB account is required). "SGDB uses an Authorization bearer token as authentication … `Authorization: Bearer <token>`".
- Relevant endpoints: `GET /games/{platform}/{platformId}` and `GET /grids/{platform}/{id*}` (look up by Steam app id, `platform` = `steam`), `GET /search/autocomplete/{term}`, `GET /grids/game/{gameId}`.
- The API doc has no terms of use, no attribution requirement and no rate limit. It actively invites apps: "If you've made one or are planning to, please let us know so we can add it here!", and lists "Projects using the API", among them game launchers and tools (Playnite SGDB Metadata Extension, GameHub, Steam ROM Manager, "clear - simple and lightweight game launcher", Steam Art Manager, a Tauri client library).
- The FAQ, Help and Privacy pages (same bundle) say nothing about the API, rate limits or commercial use.

### Assessment (inference, not legal advice)

- NiceLibrary is free and non-commercial (MIT, no ads, no server), so "personal, non-commercial use" fits each user calling the API with their own key.
- "make any automated use of the Site, the Service or the related systems" literally covers any API client. But SteamGridDB itself offers the API, issues keys per user and showcases automated clients. The clause reads as aimed at scraping and load, not at the official API. That reading is plausible but **not stated** anywhere.
- No caching or redistribution rules beyond "distribute any part … without our explicit written permission". ADR-0005 already keeps Covers only in the user's local cache and never in the repo or installer. That is consistent.
- Each user must create their own SteamGridDB account and key, matching ADR-0005 ("if the user has stored their own key there").

### Current state in the repo

- `docs/adr/0005-covers-loaded-at-runtime.md:8` (step 2 of the chain) and `:19` ("only ships once the SteamGridDB terms have been read by hand").
- No SteamGridDB code exists (`git grep -i steamgriddb` finds only ADR-0005).
- Texts that would need a change if it ships: `README.md:39` and `:117` ("connects only to the Steam Web API … and to Steam's image servers"), the About privacy text (`src/i18n/en.ts`/`de.ts`, `about.*` keys), `.github/ISSUE_TEMPLATE/feature_request.yml:8` ("talks only to Steam and GitHub"), and user story 75 in issue #1 ("contact only Steam and the update server"). SignPath's conditions also require that data sent to systems the user did not specify be described in a privacy policy, and that it can be switched off (see section 5). An opt-in source that is off by default satisfies the switch-off part.

### Open questions / must be verified by the maintainer

1. Read the terms yourself at https://www.steamgriddb.com/terms (in a browser) and confirm the text above is still current, since "We can amend these Terms of Use at any time".
2. Decide whether the API's own invitation is enough, or ask SteamGridDB in writing (their Discord or the contact address in the terms) whether a free, open-source launcher where each user stores their own key is permitted, whether there are rate limits, and whether they want attribution. A written yes turns the inference above into a fact. Record the answer in ADR-0005.
3. If built: show which host is contacted, keep it opt-in, and update README, the About text and the feature-request template.

---

## 3. Steam Web API: can the key owner read their own data while game details are private?

### What the sources say

- **Steam Web API Terms of Use** ([steamcommunity.com/dev/apiterms](https://steamcommunity.com/dev/apiterms)). Verified: "You agree to keep your Steam Web API key confidential, and not to share it with any third party." "You will only retrieve Steam Data about a Steam end user as requested by the end user." Also: "inform the end user about any Steam Data you will store", "post a privacy policy regarding the use of nonpublic end user data", "limited to one hundred thousand (100,000) calls to the Steam Web API per day", no implied endorsement or affiliation (sections 2 and 5), and "Valve may change, suspend or discontinue the Steam Web API … at any time for any reason, without notice" (section 11). Nothing here addresses privacy settings.
- **`IPlayerService/GetOwnedGames`** ([Steamworks docs](https://partner.steamgames.com/doc/webapi/IPlayerService)). Verified: "Returns a list of games owned by the player if their owned games/game details are visible to you." Parameters: `key` ("Steamworks Web API user authentication key"), `steamid`, `include_appinfo`, `include_played_free_games`, `appids_filter`. The page does not say whether "you" means the key's owner. Treat that as undocumented.
- **`ISteamUserStats/GetPlayerAchievements`** ([Steamworks docs](https://partner.steamgames.com/doc/webapi/ISteamUserStats)). Verified: no statement about privacy or visibility at all.
- **Key types** ([Web API authentication](https://partner.steamgames.com/doc/webapi_overview/auth)). Verified: user keys "are available to everyone, all that is required is a Steam account". Publisher keys "provide access to sensitive user data and protected methods" and "must not be distributed with a game client", so they are not an option for this app.
- **OAuth / access tokens** ([Steamworks OAuth](https://partner.steamgames.com/doc/webapi_overview/oauth)). Verified: "To use OAuth, first you will need to obtain a Client ID from Valve". You contact Valve about permissions, token lifetime and redirect URI. The page does not say whether a token bypasses privacy settings. An open-source desktop app without a server and without a Valve partnership cannot count on this route.
- **What the privacy settings do.** **Unverified**: the official Steam Support FAQ "Steam Profile Privacy" (https://help.steampowered.com/en/faqs/view/588C-C67D-0251-C276) loads its text by script and could not be read here. Community guides describe "Game details" (Public / Friends only / Private) and a separate checkbox "Always keep my total playtime private even if users can see my game details". Some users report that this checkbox also hides recently played games. Check the checkbox in the test as well.
- **Throwaway check already done.** Verified in `docs/adr/0002-steam-data-own-key-and-local-files.md`: for an account with private game details the API returned "almost no games, no playtime, and refused achievements with 'Profile is not public'". The ADR lists "Not yet tested: whether a key owner can read their own account while its game details are private."

### Current state in the repo

- `crates/core/src/sync.rs:90-118`: `owned_games` calls `IPlayerService/GetOwnedGames/v1` with `include_appinfo=1`, `include_played_free_games=1`. A missing `game_count` is treated as `SyncOutcome::GameDetailsPrivate` (comment at `sync.rs:101-102`: "Steam answers an account whose game details are private with an empty response").
- `crates/core/src/achievements.rs:184` (`GetPlayerAchievements/v1`), `:235` (`GetSchemaForGame/v2`).
- User story 12 in issue #1 assumes private game details block Achievements and Playtime.

### Test procedure (maintainer, own account)

Run it locally. Do not paste keys, SteamIDs or raw responses into issues, PRs or chat; write down only the observed outcome.

1. **Set up.** Use the account the key belongs to (key A on account A). Optionally use a second account B as a control. Find A's SteamID64 in the app's setup or in Steam under *Account details*.
2. **Baseline.** Profile *Public*, Game details *Public*. Run the requests below and note `game_count`, whether `playtime_forever` and `rtime_last_played` are non-zero, and whether achievements come back with `"success": true`.
3. **Private game details.** Profile *Public*, Game details *Private*, the playtime checkbox unticked. Wait a few minutes (**unverified** whether Steam caches privacy changes; repeat once later if the result is surprising). Run the same requests with **key A for SteamID A**.
4. **Friends only.** Repeat step 3 with Game details *Friends only*.
5. **Playtime checkbox.** Game details *Public* with "Always keep my total playtime private" ticked. Check `playtime_forever`, `playtime_2weeks` and `GetRecentlyPlayedGames`.
6. **Control.** With A's game details private, call with **key B for SteamID A** (and the reverse). If key A sees more of A than key B does, the owner is privileged.
7. **Restore** your privacy settings afterwards.

Requests in PowerShell. Enter the key in the session only, never in a file:

```powershell
$key = Read-Host -AsSecureString 'Steam Web API key' | ForEach-Object { [Net.NetworkCredential]::new('', $_).Password }
$id  = '<SteamID64>'
$app = '<appid of a game you own that has achievements>'
$base = 'https://api.steampowered.com'
Invoke-RestMethod "$base/IPlayerService/GetOwnedGames/v1/?key=$key&steamid=$id&include_appinfo=1&include_played_free_games=1&format=json" |
  ForEach-Object { $_.response | Select-Object game_count, @{n='withPlaytime';e={@($_.games | Where-Object playtime_forever -gt 0).Count}} }
Invoke-RestMethod "$base/IPlayerService/GetRecentlyPlayedGames/v1/?key=$key&steamid=$id&format=json" | ForEach-Object { $_.response.total_count }
try { (Invoke-RestMethod "$base/ISteamUserStats/GetPlayerAchievements/v1/?key=$key&steamid=$id&appid=$app").playerstats.success } catch { $_.ErrorDetails.Message }
Invoke-RestMethod "$base/ISteamUser/GetPlayerSummaries/v2/?key=$key&steamids=$id" | ForEach-Object { $_.response.players.communityvisibilitystate }
Remove-Variable key
```

(`Read-Host -AsSecureString` keeps the key out of the console history; the conversion to plain text exists only in memory. Do not run this with transcript logging on.)

8. **Record** in ADR-0002 under "Findings": the date, the settings of each run, and per endpoint "full / empty / refused". Then decide whether story 12's message ("this setting blocks Achievements and Playtime") holds for the key owner too, or needs rewording.

### Open questions / must be verified by the maintainer

1. The whole question is empirical. Valve's docs do not answer it.
2. If the owner *can* read their own private data, `sync.rs:101-104` stays correct for other-account keys only, and the setup hint would mislead. If the owner *cannot*, the current behaviour and texts stand.

---

## 4. Personal data in the git history

### What the sources say

- **Rewriting history** ([GitHub Docs: Removing sensitive data from a repository](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository)). Verified: the recommended tool is `git-filter-repo` (version 2.47 or later), e.g. `git-filter-repo --sensitive-data-removal --replace-text ../passwords.txt` and `git push --force --mirror origin`. Listed side effects include "Changed commit hashes", "Broken diff view for closed pull requests", "Lost signatures on commits and tags" and "Leading others directly to the sensitive data". After a force push, the commits "may still be accessible elsewhere: In any clones or forks of your repository; Directly via their SHA-1 hashes in cached views on GitHub; Through any pull requests that reference them". Only GitHub Support can "permanently remove cached views and references to the sensitive data in pull requests" ([support.github.com](https://support.github.com)).
- **git-filter-repo** ([Documentation/git-filter-repo.txt](https://github.com/newren/git-filter-repo/blob/main/Documentation/git-filter-repo.txt)). Verified: `--analyze` "Will not modify your repo". `--replace-text <expressions_file>`: literal by default, `regex:` and `glob:` prefixes, `==>` sets the replacement (default `***REMOVED***`). `--mailmap <filename>` rewrites "author, committer, and tagger names and emails". `--sensitive-data-removal` fetches all refs first, and "if you have any local-only changes … this fetch step may discard them. Working in a fresh clone avoids this problem". `--force` overrides the fresh-clone check ("an irreversible operation").
- **Secret scanners.** [gitleaks](https://github.com/gitleaks/gitleaks), verified: modes `git`, `dir`, `stdin` (`detect`/`protect` deprecated since v8.19.0). `gitleaks git -v --log-opts="--all" <repo>` passes options to `git log -p`. `--redact` hides secrets in the output; `-f json|csv|junit|sarif|template`. [trufflehog](https://github.com/trufflesecurity/trufflehog), verified: `trufflehog git file://<repo> --results=verified,unknown`; `trufflehog github --repo=<url> --issue-comments --pr-comments` also scans issue and PR comments.
- **Commit email** ([GitHub Docs: Setting your commit email address](https://docs.github.com/en/account-and-profile/setting-up-and-managing-your-personal-account-on-github/managing-email-preferences/setting-your-commit-email-address)). Verified: with "Keep my email addresses private" on, "the commit author email address cannot be changed and will be a no-reply by default" for web-based operations such as merging a PR.

### Current state (read-only check on 2026-11-27, after `git fetch --all`)

- Repo visibility: **public** (`gh api repos/SirKashij/NiceLibrary` → `"visibility": "public"`). Created 2026-11-25. 0 forks, 0 stars, 0 releases.
- Refs: 16 local branches, 19 remote-tracking branches (some are stale; the remote has 15 heads), 0 tags, 0 stashes, 0 notes. 228 commits reachable from `--all`. On GitHub: 15 `refs/heads/*` and **27 `refs/pull/*`**. The PR refs reach 11 commits that are not on any local branch.
- **Identities**, author and committer of every commit in `--all` plus all PR refs: only two distinct values. `SirKashij <258232409+SirKashij@users.noreply.github.com>` fills 429 identity slots. `GitHub <noreply@github.com>` is the committer of 27 merge commits made through the web UI, whose author is the allowed identity. **No non-allowed personal identity found.** 27 commits have GPG status `E` (GitHub's web-flow signature; the key is not in the local keyring), 201 are unsigned.
- **Content scans**, `git log --all -p -G<pattern> -i`, counted without printing the matches:
  - The maintainer's Windows user name: **1 commit, `39c1f81`** ("Keep the user's profile folder out of the Log, and tidy up the core", 2026-11-27, on `main` and pushed). It uses the name as a sample user in `crates/core/src/log.rs:200` (doc comment, an example profile path) and `crates/core/tests/log.rs:344-346, 353` (test paths for Windows, another drive and Linux, plus an assertion). It is not in any commit message and not in the extra PR-ref commits.
  - `C:\Users\`: the same commit `39c1f81` only. No `C:/Users/` or `/c/Users/` paths.
  - `AppData`: 6 commits. These are documentation of `%APPDATA%`/`%LOCALAPPDATA%` and test code, with no user names (checked via the user-name scan above).
  - Personal mail domains (gmail, outlook, web.de, gmx): 0. `BEGIN … PRIVATE KEY`: 0. No `.env`, `.db`, `.pfx` or `.p12` file was ever committed (`git log --all --name-only`).
  - 32-digit hex values (key-shaped): only 3 distinct ones ever added, all synthetic patterns (`0123456789…`, `AAAA…`, `FEDCBA98…`).
  - SteamID64-shaped values: 5 distinct ones, all synthetic test values near the base `76561197960265728`. **Neither of the 2 SteamIDs in the local Steam `loginusers.vdf` appears anywhere in the history or the current tree.**
- `.claude/settings.json` is tracked. It holds a hook command with `$CLAUDE_PROJECT_DIR`, not a local path.

Commands to repeat the audit (they print counts and SHAs, not values):

```bash
A='SirKashij <258232409+SirKashij@users.noreply.github.com>'
git fetch origin '+refs/pull/*:refs/remotes/origin/pr/*'          # include PR refs
git log --all --format='%H %an <%ae>|%cn <%ce>' | awk -v a="$A" -F'|' '
  { split($1,x," "); id=substr($1,length(x[1])+2)
    if (id!=a || ($2!=a && $2!="GitHub <noreply@github.com>")) print x[1] }'   # prints SHAs of non-allowed identities
git log --all -i -G'C:\\\\Users\\\\|/home/|/Users/' --format=%h         # profile paths
git log --all -i -S"$(basename "$HOME")" --format=%h                     # your user name
git log --all -E -G'[0-9A-Fa-f]{32}|7656119[0-9]{10}' --format=%h        # key- and SteamID-shaped values
gitleaks git -v --redact --log-opts="--all" .
```

### Open questions / must be verified by the maintainer

1. **Visibility versus ADR-0006.** Was going public before v0.1.0 intended (for example because environment protection rules need a public repo on the free plan, **unverified**)? If yes, update ADR-0006's last consequence. If not, decide whether to make it private again. That hides the repo, but anything already fetched or cached stays out there.
2. **The user name in `39c1f81`.** It is the Windows user name, which the global rule excludes from public repos. It is a common first name and appears without context, but it is still personal data by that rule. Options:
   - (a) Replace it with a neutral name (e.g. `alice`) in a new commit. The old commit stays reachable by SHA, through `refs/pull/*` and in GitHub's caches.
   - (b) Additionally rewrite the history with `git filter-repo --sensitive-data-removal --replace-text` in a fresh clone, force-push all branches, and ask GitHub Support to purge cached views and PR refs. All SHAs change, and the merged PR diffs break.
   Whether (b) is worth it is a judgement call, since the repo has 0 forks and was public for about two days.
3. Stale local branches and remote-tracking refs (`git remote prune origin`) do not affect what is public, but clean them up before the next audit so the counts match.
4. Also check what history cannot show: screenshots in issues and PRs, issue and PR comments (`trufflehog github --repo=… --issue-comments --pr-comments`), release assets (none yet), and the GitHub profile itself.
5. Keep "Keep my email addresses private" and "Block command line pushes that expose my email" switched on in the GitHub settings, so future web merges stay on the noreply address.

---

## 5. SignPath Foundation (after the first release)

### What the sources say

**Conditions** ([signpath.org/terms](https://signpath.org/terms)). Verified, headings and requirements in order:

- *Free OSS subscriptions*: no malware or potentially unwanted programs. "The project must use an OSI-approved Open Source license without commercial dual-licensing for all components." No proprietary components (system libraries are allowed). "Actively maintained". "The project must already be released in the form that should be signed." Functionality described "on its download page or in the app store entry".
- *What can be signed*: only your own projects and your own binaries, built from your own source. Upstream binaries may be included unsigned. No hacking tools.
- *End user interactions*: software that transfers data to systems the user has not specified must describe this in a privacy policy, display it during installation, and offer options to disable it. "Software must not modify the user's system configuration without proper warnings." Provide uninstallation.
- *Contributors*: "All team members must use multi-factor authentication for both SignPath and source code repository access." Assign roles: authors, reviewers (changes from non-committers reviewed by a team member), approvers ("each signing request must be approved by a team member trusted by the entire team").
- *Website / repository*: "A code signing policy must be specified on the project's home page", using the term "Code signing policy" on the home page and the download or release pages. It must include the attribution "Free code signing provided by SignPath.io, certificate by SignPath Foundation", the team roles and their members, and a privacy policy link or a statement that the program won't transfer information to other networked systems unless the user requests it.
- *SignPath configuration*: artifact configurations enforce metadata restrictions. The **product name must match the project name**, and the product version must be the same in each build.
- *Other*: "Don't fight the system" (verifiable builds, manual approval per release). Help with investigations.
- *Common misunderstandings*: SignPath does not issue certificates to projects, and "projects have no right to a certificate". The Foundation verifies "project reputation and control" first.
- **Application:** [signpath.org/apply](https://signpath.org/apply) is a form ("Apply for a free SignPath.io subscription") embedded from HubSpot. Its fields could not be read here (**unverified** what it asks).

**GitHub integration** ([docs.signpath.io: GitHub](https://docs.signpath.io/trusted-build-systems/github)). Verified: link the predefined Trusted Build System "GitHub.com" to the SignPath project. The artifact "Must be uploaded using the actions/upload-artifact v4+ action before it can be signed", and its id goes to `github-artifact-id`. For OSS projects, "All jobs of the GitHub workflow leading up to the signing request were executed on GitHub-hosted agents". Inputs of `signpath/github-action-submit-signing-request`: `api-token`, `organization-id`, `project-slug`, `signing-policy-slug`, `github-token` (`actions: read`, `contents: read`), `output-artifact-directory`. Origin verification: "Origin metadata is provided by GitHub, not the build script, and can therefore not be forged". Optional policies need GitHub branch rulesets (`github-scm-policies`) and can forbid re-runs (`disallow_reruns`).

### Current state in the repo

- `docs/adr/0006-unsigned-first-release-then-signpath.md`: apply after v0.1.0, CI built to the conditions, manual approval, history checked before going public.
- `.github/workflows/release.yml:5-16` describes where the signing job goes. The build runs on `windows-latest` (GitHub-hosted, `:43`). The artifact is uploaded with `actions/upload-artifact@v7` (`:86`, at least the required v4) and its `artifact-id` is exported (`:41-42`). Publishing needs the `release` environment (`:99`), which has `required_reviewers` and `branch_policy` (GitHub API). The tagged commit must be on `main` (`:51-56`). The workflow uses no caches (`:64-65`).
- Branch ruleset `main` is active (GitHub API), which SignPath's `github-scm-policies` can build on.
- License: MIT (`LICENSE`, ADR-0004); `deny.toml` enforces dependency licenses.
- `README.md:31` and `:109` announce SignPath for later releases. `CONTRIBUTING.md:110` explains where signing fits.
- Missing for SignPath: a "Code signing policy" section in the README (the home page) with the attribution sentence, roles and members, and a privacy statement. A privacy text exists (`README.md:39`) and can be linked. The installer has no step that displays the privacy notice. Whether that is needed depends on whether the update check counts as "systems the user has not specified" (**unverified**; the switch in the settings covers "options to disable").

### Open questions / must be verified by the maintainer

1. Apply after v0.1.0 is published under the **final** name, because the product name in the binaries must match the project name.
2. Turn on MFA for GitHub (and later SignPath) if it is not already on.
3. Before applying, add the "Code signing policy" section to the README (English and German) and to the release page template (`release.yml:205` builds the release title and notes).
4. Decide whether the update check (`crates/core/src/updates.rs`, uncommitted) needs to be mentioned in the installer per the end-user-interaction condition, or whether the README privacy section and the settings switch are enough. Ask SignPath when applying if unsure.
5. Decide whether to also sign `nicelibrary.exe` inside the installer (`release.yml:14-16` describes the split build), or only the installer.
