# Contributing to NiceLibrary

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
cargo install tauri-driver --locked
pwsh scripts/install-msedgedriver.ps1   # again after each WebView2 update
```

The second command puts the msedgedriver that matches your WebView2 runtime into `target/e2e/`.

Then build the app and run the tests:

```sh
npm run tauri -- build --debug --no-bundle
npm run test:e2e
```

The tests use the most recently built of `target/release/nicelibrary.exe` and `target/debug/nicelibrary.exe`. Set `NICELIBRARY_APP` to test another binary.

### Adding an end-to-end check

Add one `it(...)` to `e2e/smoke.e2e.ts`, or create a new `e2e/<topic>.e2e.ts` that calls `useApp()` the same way. `useApp()` launches the app once for all tests in the file and closes it afterwards. The handle it returns has:

- `text(selector)`: the text of the first element that matches a CSS selector, waiting until it appears;
- `messages()`: the interface texts in the language the app chose, so assertions work on German and English systems;
- `execute(script, ...args)`: runs a script in the app's window and returns its result.

If a check needs another WebDriver command, add a method to the handle in `e2e/app.ts`, not raw requests to a test.
