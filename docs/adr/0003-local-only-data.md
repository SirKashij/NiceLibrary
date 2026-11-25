# All data stays on the user's computer

The project is run by a private individual and is meant to stay free of legal and operating burden, so there is no server, no account, no cloud sync and no telemetry. Everything is stored in a local SQLite database in the user profile; backup is an export/import file.

## Consequences

- The only outbound connections are the ones the user sets up or can switch off: Steam (API and artwork) and the update check against GitHub.
- No automatic crash reports. Errors go to a local log file the user can attach to an issue voluntarily.
- Any feature that would need a shared secret or a relay (a shared API key, a metadata proxy) is out of scope by this decision, which is why every external service is used with the user's own credentials.
- No donation buttons or sponsoring in version 1, so the project stays clearly private and non-commercial (German imprint rules).
