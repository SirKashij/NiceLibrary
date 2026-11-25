# The first release is unsigned; SignPath Foundation signing follows

Windows warns about unsigned installers, but the affordable routes are closed: Microsoft's Artifact Signing does not serve individuals in Germany, and a purchased certificate costs several hundred euros a year. SignPath Foundation signs open-source projects for free, but only after a first public release. So v0.1.0 ships unsigned through GitHub Releases, and we apply to SignPath right after.

## Consequences

- CI is built to SignPath's conditions from the start: releases are built only from source in CI and each one is approved by hand.
- The README explains the SmartScreen warning and lists checksums. Update packages are signed with the Tauri updater's own key regardless.
- The repo stays private until v0.1.0. Before it goes public, the whole history is checked for personal data.
