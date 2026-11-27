# NiceLibrary

_Working title._

**English** · [Deutsch](#deutsch)

A free, open-source Windows desktop app that gathers your games from Steam and from your own disk into one library, tracks how much and how far you have played, and launches them. There is no server, no account and no telemetry: all data stays on your computer.

## What it does

- Shows all Games of your Steam Account, Installed or not, together with Local Games you add by pointing at their executable, as one Library of Covers.
- Reads your Steam Games, Playtime and Achievements with your own Steam Web API key. Without a key it runs in Limited Mode: your Installed Steam Games still show up and launch, Achievements are missing.
- Lets you search, filter and sort the Library, group Games into Collections and set a Status on each Game.
- Launches Steam Games through Steam itself, so the overlay, cloud saves and Steam's own playtime keep working, and Local Games by starting their executable.
- Keeps running in the Tray when you close the window, so tracking goes on.
- Exports your own data into one file, as a backup or to move to another computer, and imports it there; your Steam Web API key is never in it ([format](docs/export-format.md)).
- Speaks English and German, in a dark or a light Theme.

The app is in early development; there is no release yet.

## Installing

Releases will be published on [GitHub Releases](https://github.com/SirKashij/NiceLibrary/releases) as an installer for Windows 10 (22H2) and 11, 64-bit. Listing the SHA-256 checksum of the installer with each release is planned, but the release process that publishes it is not set up yet.

The first release is not code-signed (see [ADR-0006](docs/adr/0006-unsigned-first-release-then-signpath.md)), so Windows SmartScreen will probably warn with "Windows protected your PC" when you start the installer. Once a release page lists the checksum, you can check that you have the genuine file by comparing it with the checksum of your download:

```powershell
Get-FileHash .\NiceLibrary_0.1.0_x64-setup.exe -Algorithm SHA256
```

If the two match, choose **More info**, then **Run anyway**. If they do not match, do not run the file. Later releases are meant to be signed through the SignPath Foundation, which removes the warning.

## Privacy

NiceLibrary connects only to the Steam Web API, for your games, playtime and achievements, and to Steam’s image servers, for covers.

Everything it stores stays only on this computer: your library and settings in a database in your user profile, covers in a cache folder, and your Steam Web API key in the Windows Credential Manager.

Nothing else leaves your computer.

In detail:

| Connection | What for |
| --- | --- |
| `api.steampowered.com` | The Steam Web API, with your own key: your Games, Playtime and Achievements during a Sync, a check of a newly entered key, and Steam's app list for Cover Matches |
| `shared.steamstatic.com` | Steam's image servers: the Covers of your Games |

The page where you create a Steam Web API key opens in your own browser; the app does not load it. Covers are downloaded at runtime and kept only in the cache on your computer; no game artwork is part of this repo or the installer.

## Building from source

Requirements: Windows, Node.js 22 or newer, a Rust toolchain, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/). The app also builds on Linux, but version 1 ships only for Windows.

```sh
npm install
npm run dev                   # starts the app with live reload
npm run tauri -- build        # builds the installer into target/release/bundle/nsis/
```

CI builds and tests every pull request and every push to `main` on Windows and builds the app on Linux, without publishing anything.

## Contributing

Contributions are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) explains the terms (MIT, no contributor license agreement) and how to run the tests; the [code of conduct](CODE_OF_CONDUCT.md) applies everywhere in the project. Report security problems privately as described in [SECURITY.md](SECURITY.md).

## Trademarks

Steam and the Steam logo are trademarks and/or registered trademarks of Valve Corporation in the U.S. and/or other countries.

NiceLibrary is not affiliated with, endorsed or sponsored by Valve Corporation.

## License

[MIT](LICENSE)

---

## Deutsch

Eine kostenlose, quelloffene Windows-App, die deine Spiele aus Steam und von deiner eigenen Festplatte in einer Bibliothek zusammenführt, festhält, wie viel und wie weit du gespielt hast, und sie startet. Es gibt keinen Server, kein Konto und keine Telemetrie: Alle Daten bleiben auf deinem Rechner.

### Was die App kann

- Sie zeigt alle Spiele deines Steam-Kontos, installiert oder nicht, zusammen mit lokalen Spielen, die du über ihre ausführbare Datei hinzufügst, als eine Bibliothek aus Covern.
- Sie liest deine Steam-Spiele, deine Spielzeit und deine Trophäen mit deinem eigenen Steam-Web-API-Schlüssel. Ohne Schlüssel läuft sie im eingeschränkten Modus: Deine installierten Steam-Spiele erscheinen und starten trotzdem, nur die Trophäen fehlen.
- Du kannst die Bibliothek durchsuchen, filtern und sortieren, Spiele in Sammlungen ordnen und jedem Spiel einen Status geben.
- Steam-Spiele startet sie über Steam selbst, damit Overlay, Cloud-Speicherstände und Steams eigene Spielzeit weiter funktionieren, lokale Spiele über ihre ausführbare Datei.
- Wenn du das Fenster schließt, läuft sie im Infobereich weiter, damit die Aufzeichnung nicht abbricht.
- Sie exportiert deine eigenen Daten in eine Datei, als Sicherung oder für den Umzug auf einen anderen Computer, und importiert sie dort wieder; dein Steam-Web-API-Schlüssel ist nie darin ([Format](docs/export-format.md)).
- Sie spricht Deutsch und Englisch, in einem dunklen oder hellen Design.

Die App ist in früher Entwicklung; es gibt noch kein Release.

### Installieren

Releases erscheinen auf [GitHub Releases](https://github.com/SirKashij/NiceLibrary/releases) als Installer für Windows 10 (22H2) und 11, 64 Bit. Geplant ist, zu jedem Release die SHA-256-Prüfsumme seines Installers anzugeben; der Release-Ablauf, der sie veröffentlicht, ist aber noch nicht eingerichtet.

Das erste Release ist nicht codesigniert (siehe [ADR-0006](docs/adr/0006-unsigned-first-release-then-signpath.md)), deshalb warnt Windows SmartScreen beim Start des Installers wahrscheinlich mit „Der Computer wurde durch Windows geschützt“. Sobald die Release-Seite die Prüfsumme angibt, kannst du prüfen, ob du die echte Datei hast, indem du sie mit der Prüfsumme deines Downloads vergleichst:

```powershell
Get-FileHash .\NiceLibrary_0.1.0_x64-setup.exe -Algorithm SHA256
```

Stimmen beide überein, wähle **Weitere Informationen** und dann **Trotzdem ausführen**. Stimmen sie nicht überein, führe die Datei nicht aus. Spätere Releases sollen über die SignPath Foundation signiert werden; dann entfällt die Warnung.

### Datenschutz

NiceLibrary verbindet sich nur mit der Steam Web API für deine Spiele, Spielzeit und Trophäen und mit den Bildservern von Steam für Cover.

Alles, was es speichert, bleibt nur auf diesem Computer: deine Bibliothek und Einstellungen in einer Datenbank in deinem Benutzerprofil, Cover in einem Cache-Ordner und dein Steam-Web-API-Schlüssel in der Windows-Anmeldeinformationsverwaltung.

Sonst verlässt nichts deinen Computer.

Im Einzelnen:

| Verbindung | Wofür |
| --- | --- |
| `api.steampowered.com` | Die Steam Web API mit deinem eigenen Schlüssel: deine Spiele, Spielzeit und Trophäen bei einer Synchronisierung, die Prüfung eines neu eingegebenen Schlüssels und Steams App-Liste für die Cover-Zuordnung |
| `shared.steamstatic.com` | Die Bildserver von Steam: die Cover deiner Spiele |

Die Seite, auf der du einen Steam-Web-API-Schlüssel anlegst, öffnet sich in deinem eigenen Browser; die App lädt sie nicht. Cover werden zur Laufzeit heruntergeladen und nur im Cache auf deinem Rechner abgelegt; kein Spielbild ist Teil dieses Repos oder des Installers.

### Aus dem Quellcode bauen

Voraussetzungen: Windows, Node.js 22 oder neuer, eine Rust-Toolchain und die [Tauri-2-Voraussetzungen](https://v2.tauri.app/start/prerequisites/). Die App lässt sich auch unter Linux bauen, Version 1 erscheint aber nur für Windows.

```sh
npm install
npm run dev                   # startet die App mit Live-Reload
npm run tauri -- build        # baut den Installer nach target/release/bundle/nsis/
```

Die CI baut und testet jeden Pull Request und jeden Push auf `main` unter Windows und baut die App unter Linux, ohne etwas zu veröffentlichen.

### Mitmachen

Beiträge sind willkommen. [CONTRIBUTING.md](CONTRIBUTING.md) (auf Englisch) erklärt die Bedingungen (MIT, keine Contributor License Agreement) und wie du die Tests ausführst; der [Verhaltenskodex](CODE_OF_CONDUCT.md) gilt überall im Projekt. Sicherheitsprobleme meldest du vertraulich, wie in [SECURITY.md](SECURITY.md) beschrieben.

### Marken

Steam und das Steam-Logo sind Marken und/oder eingetragene Marken der Valve Corporation in den USA und/oder anderen Ländern.

NiceLibrary steht in keiner Verbindung zur Valve Corporation und wird von ihr weder unterstützt noch gesponsert.

### Lizenz

[MIT](LICENSE)
