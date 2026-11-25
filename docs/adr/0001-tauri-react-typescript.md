# Tauri 2 with a React and TypeScript frontend

The app has to look modern and also sit in the tray all day watching game processes, so it needs free-form UI and a small idle footprint. We build it with Tauri 2: the interface in React and TypeScript, everything that touches the operating system in Rust.

## Considered Options

- **Electron**: same UI freedom and a single language, but a 150 MB+ installer and noticeably more memory for a process that runs permanently.
- **WinUI 3 / WPF**: native like Playnite, but a modern look is much more manual work and it rules out Linux and macOS.

## Consequences

- Version 1 ships for Windows 10 (22H2) and Windows 11, 64-bit only. Linux and macOS are planned afterwards.
- To keep that door open, everything OS-specific lives in the backend behind its own interface: process detection, locating Steam, launching, secret storage, autostart, tray. The data model contains nothing Windows-only ("executable", not "EXE"; no drive-letter assumptions). CI also builds on Linux from day one without publishing.
