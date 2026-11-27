# Security policy

## Reporting a vulnerability

Please do not report security problems in a public issue.

Report them privately through GitHub instead: open the
[Security tab](https://github.com/SirKashij/NiceLibrary/security) and choose
**Report a vulnerability**. Only the maintainers can read the report.

Helpful details are the version or commit you used, what an attacker can do,
and the steps to reproduce it.

## Supported versions

NiceLibrary has no release yet. Fixes land on `main`.

## What counts

NiceLibrary is a desktop app that reads local Steam files and calls the Steam
Web API with a key the user enters. Problems with how the app stores that key,
handles files or talks to the network are in scope. Problems in Steam itself
are not; report those to Valve.
