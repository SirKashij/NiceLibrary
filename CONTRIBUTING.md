# Contributing to NiceLibrary

## License of contributions

NiceLibrary is MIT licensed (ADR-0004). By contributing, you agree that your contribution is licensed under the same MIT license ("inbound = outbound"). There is no contributor license agreement.

## Tests

| Command | What it tests |
| --- | --- |
| `npm test` | The interface against a fake of the core's operations (`src/**/*.test.tsx`) |
| `cargo test` | The application core through its operations |
| `cargo test-contract` | The real Windows ports (Windows only) |
| `npm run test:e2e` | The built app, driven through WebDriver (Windows only) |

CI runs all of them on Windows for every pull request.

### End-to-end tests

The end-to-end tests in `e2e/` start the built app and check what no lower test can see: that the window opens and the interface reaches the core through the real Tauri commands. They are slow, so keep them few and leave everything else to the tests above.

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

### When the end-to-end tests fail

A failure in `beforeAll` (no session) means the app did not come up under WebDriver. `scripts/collect-e2e-diagnostics.ps1` records the WebView2, Edge and driver versions, the browser arguments that reached the app's WebView2, and a verbose msedgedriver log in `target/e2e/diagnostics/`. CI runs it on every end-to-end failure and attaches the folder to the run as the `e2e-diagnostics` artifact (`gh run download <run-id> -n e2e-diagnostics`).

To try a CI change without a pull request, push it to a scratch branch and run `gh workflow run CI --ref <branch>`.

Stop only processes you started, by PID (`taskkill /PID <pid> /T /F`). Stopping by name also stops other apps that share the name, such as every WebView2 app on the machine. A Claude Code hook in `.claude/settings.json` refuses commands that stop processes by name.

### Adding an end-to-end check

Add one `it(...)` to `e2e/smoke.e2e.ts`, or create a new `e2e/<topic>.e2e.ts` that calls `useApp()` the same way. `useApp()` launches the app once for all tests in the file and closes it afterwards. The handle it returns has:

- `text(selector)`: the text of the first element that matches a CSS selector, waiting until it appears;
- `messages()`: the interface texts in the language the app chose, so assertions work on German and English systems;
- `execute(script, ...args)`: runs a script in the app's window and returns its result.

If a check needs another WebDriver command, add a method to the handle in `e2e/app.ts`, not raw requests to a test.
