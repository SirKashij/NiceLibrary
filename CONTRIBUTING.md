# Contributing to NiceLibrary

Thanks for helping. Everyone who takes part follows the [code of conduct](CODE_OF_CONDUCT.md).

## How to take part

- Report bugs and suggest features through the [issue templates](https://github.com/SirKashij/NiceLibrary/issues/new/choose). Report security problems privately instead (see [SECURITY.md](SECURITY.md)).
- For anything bigger than a small fix, open or comment on an issue first, so we can agree on the behaviour before you write code.
- Open one pull request per change, against `main`. CI must pass before it is merged.

Code, comments, commit messages, issues and pull requests are in English. The interface and the README are in English and German: every interface text goes through the translation system (`src/i18n/en.ts` and `src/i18n/de.ts`), and a change to the README changes both parts. Use the terms in [GLOSSARY.md](GLOSSARY.md); decisions are recorded in [docs/adr/](docs/adr/).

## License of contributions

NiceLibrary is MIT licensed (ADR-0004). By contributing, you agree that your contribution is licensed under the same MIT license ("inbound = outbound"). There is no contributor license agreement.

## No artwork, keys or personal data

Game artwork belongs to its publishers (ADR-0005). Never commit a game image, and use placeholder Covers in every screenshot, including those in the README, issues and pull requests. Never commit a Steam Web API key, a SteamID or other personal data, and keep them out of screenshots and logs you attach.

## Tests

| Command | What it tests |
| --- | --- |
| `npm test` | The interface against a fake of the core's operations (`src/**/*.test.tsx`) |
| `cargo test` | The application core through its operations |
| `cargo test-contract` | The real Windows ports (Windows only) |
| `npm run test:e2e` | The built app, driven through WebDriver (Windows only) |

CI runs all of them on Windows for every pull request.

`cargo test-contract` writes temporary registry values under `HKEY_CURRENT_USER\Software\NiceLibraryContractTests` and removes them when it finishes. It only reads Steam's own registry keys. To check starting with Windows, it writes its test startup entries there too, in place of Windows' own `Run` and `StartupApproved` keys; it only reads the app's real startup entry and never adds or removes it. To check how URLs are opened, it registers URL schemes of its own under `HKEY_CURRENT_USER\Software\Classes\nicelibrary-contract-*`, opens them (a console window flashes briefly) and removes them again; it never opens a `steam://` URL. To check how folders are shown, it opens one File Explorer window on an empty temporary folder, which stays open. To check the secret store, it writes test entries to the Windows Credential Manager under `NiceLibraryContractTests/` and deletes them again; it never touches the app's own entries (`io.github.sirkashij.nicelibrary/`). To check the file dialogs for a Local Game's executable, for a Cover Image and for a folder of Games, it opens Windows' "Open" and "Select Folder" dialogs a few times (each shows on the screen for a moment), types a path to a temporary file or folder into it or cancels it; for an Export File it does the same with Windows' "Save as" dialog, typing a path in a temporary folder, where nothing is written. Do not click into the dialogs while the tests run. To check the process list, it copies its own test program into a temporary folder, starts the copy a few times as a stand-in game that only waits and ends those copies again; it stops no other process. To check how a Local Game's executable is started, it starts the contract test executable itself a few times in temporary folders, where it only records the arguments and folder it was started with; it starts no other program. To check how an executable's icon is read, it copies Windows' `cmd.exe` into a temporary folder, replaces the copy's resources with icons of its own and reads them back, and reads the icon of Windows' own `explorer.exe`; the originals are only read and nothing is started. It also starts small web servers on `127.0.0.1`, downloads one Cover from Steam's artwork host and asks the Steam Web API twice with a made-up key (once to check a key, once as a Sync would), so it needs a network connection.

### End-to-end tests

The end-to-end tests in `e2e/` start the built app and check what no lower test can see: that the window opens, the interface reaches the core through the real Tauri commands, closing the window keeps the app running in the Tray and quitting from the Tray ends it. They are slow, so keep them few and leave everything else to the tests above.

One-time setup:

```sh
cargo install tauri-driver --version 2.1.0 --locked   # the version CI uses
powershell -ExecutionPolicy Bypass -File scripts/install-msedgedriver.ps1   # again after each WebView2 update
```

The second command puts the msedgedriver that matches your WebView2 runtime into `target/e2e/`.

Then build the app and run the tests:

```sh
npm run tauri -- build --debug --no-bundle
npm run test:e2e
```

The tests use the most recently built of `target/release/nicelibrary.exe` and `target/debug/nicelibrary.exe`. Set `NICELIBRARY_APP` to test another binary.

The app runs only once per user: a second start brings the running one forward and ends. Quit NiceLibrary from its Tray menu before you run the end-to-end tests, or the app under test ends at once and no session starts.

`e2e/tray.e2e.ts` closes the window, starts the app a second time and quits it from the Tray menu. It does so with window messages (`e2e/windows.ps1`) and picks the menu entry with the keyboard, so do not use mouse or keyboard while it runs. Closing the window for the first time marks the Tray notice as shown in your database, so the app does not show you the notice afterwards. With close-to-tray turned off in your settings, the closing test is skipped.

### When the end-to-end tests fail

A failure in `beforeAll` (no session) means the app did not come up under WebDriver. `scripts/collect-e2e-diagnostics.ps1` records the WebView2, Edge and driver versions, the browser arguments that reached the app's WebView2, and a verbose msedgedriver log in `target/e2e/diagnostics/`. CI runs it on every end-to-end failure and attaches the folder to the run as the `e2e-diagnostics` artifact (`gh run download <run-id> -n e2e-diagnostics`).

To try a CI change without a pull request, push it to a scratch branch and run `gh workflow run CI --ref <branch>`.

Stop only processes you started, by PID (`taskkill /PID <pid> /T /F`). Stopping by name also stops other apps that share the name, such as every WebView2 app on the machine. A Claude Code hook in `.claude/settings.json` refuses commands that stop processes by name.

### Adding an end-to-end check

Add one `it(...)` to `e2e/smoke.e2e.ts`, or create a new `e2e/<topic>.e2e.ts` that calls `useApp()` the same way. `useApp()` launches the app once for all tests in the file and closes it afterwards. The handle it returns has:

- `text(selector)`: the text of the first element that matches a CSS selector, waiting until it appears;
- `textIn(labelId, selector)`: the same, but only inside the region labelled `labelId`, the element with `aria-labelledby="<labelId>"`, such as a settings section. `:scope` in the selector stands for the region, so `textIn('settings-steam', ':scope > :not(h2)')` is the first line under the Steam heading. Use it whenever a check looks inside a section: a selector over the whole page can match an element of another section and pass by luck;
- `messages()`: the interface texts in the language the app chose, waiting until it has chosen one, so assertions work on German and English systems;
- `execute(script, ...args)`: runs a script in the app's window and returns its result.

If a check needs another WebDriver command, add a method to the handle in `e2e/app.ts`, not raw requests to a test.

## The database

The app keeps its data in one SQLite database in the user profile, `%APPDATA%\io.github.sirkashij.nicelibrary\nicelibrary.db`. The end-to-end tests start the real app, so they use that file too. The core tests use a temporary database each.

The schema changes only through migrations, the list `MIGRATIONS` in `crates/core/src/store.rs`. To change the schema, append an entry; never edit or reorder one that has been released. A database written by a newer version of the app is refused, not changed.

The Steam Web API key is never kept in the database, a file or a log. It lives only in the Windows Credential Manager, as the generic credential `io.github.sirkashij.nicelibrary/steam-web-api-key` (ADR-0002). Error messages never repeat it, nor a URL that carries it.

## The log

The app writes errors and notable events (Syncs, Launches, a key stored or removed) to `%APPDATA%\io.github.sirkashij.nicelibrary\logs\nicelibrary.log`. When that file reaches 1 MiB it becomes `nicelibrary.1.log`, the one before `nicelibrary.2.log`, and the oldest is dropped, so the log never takes more than 3 MiB. Before an entry is written, the core replaces every run of 32 or more hexadecimal digits (a Steam Web API key) with `[key]` and every run of 17 or more digits (a SteamID64) with `[SteamID]` (`crates/core/src/log.rs`). The app never sends the log; the settings only open its folder, so the user attaches a file to a bug report themselves. The end-to-end tests start the real app, so they write to that log too.
