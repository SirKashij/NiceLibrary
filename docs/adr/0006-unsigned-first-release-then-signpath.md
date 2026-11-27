# The first release is unsigned; SignPath Foundation signing follows

Windows warns about unsigned installers, but the affordable routes are closed: Microsoft's Artifact Signing does not serve individuals in Germany, and a purchased certificate costs several hundred euros a year. SignPath Foundation signs open-source projects for free, but only after a first public release. So v0.1.0 ships unsigned through GitHub Releases, and we apply to SignPath right after.

## Consequences

- CI is built to SignPath's conditions from the start: releases are built only from source in CI and each one is approved by hand.
- The README explains the SmartScreen warning and lists checksums. Update packages are signed with the Tauri updater's own key regardless.
- The repo is public before v0.1.0. Its whole history was checked for personal data on 2026-11-27: every author and committer is the project's noreply identity or GitHub's web-flow committer, and the one sample user name in the code was replaced in a new commit rather than by rewriting the history.
