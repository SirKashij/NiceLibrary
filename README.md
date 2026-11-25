# NiceLibrary

_Working title._

**English** · [Deutsch](#deutsch)

A Windows desktop app that gathers your games from Steam and from your own disk into one library, tracks how much and how far you have played, and launches them. All data stays on your computer.

The app is in early development; there is no release yet.

## Development

Requirements: Node.js 22 or newer, a Rust toolchain, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your system.

```sh
npm install
npm run dev     # starts the app
npm test        # interface tests
cargo test      # core tests
cargo test-contract   # real ports, Windows only
```

CI builds and tests every pull request and every push to `main` on Windows and builds the app on Linux, without publishing anything.

## License

[MIT](LICENSE)

---

## Deutsch

Eine Windows-App, die deine Spiele aus Steam und von deiner eigenen Festplatte in einer Bibliothek zusammenführt, festhält, wie viel und wie weit du gespielt hast, und sie startet. Alle Daten bleiben auf deinem Rechner.

Die App ist in früher Entwicklung; es gibt noch kein Release.

### Entwicklung

Voraussetzungen: Node.js 22 oder neuer, eine Rust-Toolchain und die [Tauri-2-Voraussetzungen](https://v2.tauri.app/start/prerequisites/) für dein System.

```sh
npm install
npm run dev     # startet die App
npm test        # Tests der Oberfläche
cargo test      # Tests des Core
cargo test-contract   # echte Ports, nur Windows
```

Die CI baut und testet jeden Pull Request und jeden Push auf `main` unter Windows und baut die App unter Linux, ohne etwas zu veröffentlichen.

### Lizenz

[MIT](LICENSE)
