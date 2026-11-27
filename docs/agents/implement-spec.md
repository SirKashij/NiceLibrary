# Running implement-spec in this repo

What the first run (spec #1, v0.1, 41 tickets) learned. The orchestrator points each subagent at its brief below instead of copying it into the prompt.

## Orchestrator

- **Integration branch**: `spec-<number>-<slug>`, pushed to origin. Open the draft pull request to `main` after the first merge.
- **Notes**: keep the run's notes in a `notes` folder in the session scratchpad and give each subagent the path. Each subagent gets its own folder `work/<role-or-ticket>/` beside it for temporary files.
- **Merging is the bottleneck.** One merger runs at a time. Start it as soon as a branch is finished, and hand it every branch that is waiting (up to five) in one go. While more than five branches wait, start no new implementer.
- **Chains**: a ticket that builds on an unmerged ticket starts from that ticket's branch. Merge the chain once, through its last branch.
- **CI after every merge group**: wait for the CI run of the group's last push (`gh run watch`) and get it green before the next group starts. Superseded runs on a pull request are cancelled, so only that last push gets a full run, and it is the only place the end-to-end tests run.
- **Cross-ticket duties**: keep a `integration-duties.md` in the notes. Whenever an implementer reports something that only becomes due once another ticket lands (a new per-Game table that removing a Local Game must clear, a view that must leave out Hidden Games), add it there. Mergers work through it.
- **Follow-ups**: collect open points from the reports in the notes. Before the pull request is marked ready, every open point is an issue labelled `needs-triage` or a line in the pull request the user has seen.

## Implementer brief (one ticket)

Read first: your ticket (`gh issue view <N> --json title,body,comments`), the spec sections it cites, `CONTRIBUTING.md`, `GLOSSARY.md` and `docs/adr/`.

1. `git fetch origin`. HEAD must contain the integration branch (`git merge-base --is-ancestor origin/<integration> HEAD`); if not, `git reset --hard origin/<integration>`. Create `issue-<N>-<short-slug>`.
2. Build the ticket test-first with the `mattpocock-skills:tdd` skill. Every acceptance criterion is met, or the report names the one that is not.
3. Put the work in new modules and files. The [hotspots](#hotspots) are edited by every ticket in parallel: touch them with the smallest possible change and append to them.
4. Done means green: `npm.cmd test`, `npm.cmd run typecheck`, `cargo test --workspace`, and `cargo test-contract` when the branch touches `src-tauri/src/ports` or `src-tauri/tests/contract.rs`. The end-to-end tests run in CI.
5. Before reporting: `git fetch origin`, `git merge origin/<integration>`, resolve, run the checks again, commit.
6. Leave pushing, pull requests and issue comments to the orchestrator.

Report: branch, tip SHA, two to four lines on what was built, any criterion not met, any duty another ticket inherits, any follow-up worth an issue.

## Merger brief

Work in the main checkout on the integration branch. You are the only merger.

1. Clean `git status`, then `git pull --ff-only origin <integration>` and `git merge --no-ff origin/main`.
2. Per branch, in the order given: `git merge --no-ff <branch> -m "Merge <branch>: <ticket title> (#<N>)"`.
3. Resolve conflicts so both sides' behaviour survives. Read the auto-merged [hotspots](#hotspots) too: Git has silently dropped one side's call there before.
4. Fulfil every duty in `integration-duties.md` that this merge makes due, with a test.
5. Run the implementer's checks from step 4 of its brief. Fix what the merge broke in a follow-up commit. A branch that is broken on its own: `git merge --abort` and report.
6. Authors: `git log origin/main..HEAD --format='%an <%ae> | %cn <%ce>'` lists only the allowed identity. Anything else: stop and report without pushing.
7. `git push origin <integration>` after each branch.

Report: merged SHAs, conflicts resolved, duties fulfilled, anything broken.

## Shell habits that get refused here

- **git**: one plain git command per call, run from the agent's own working directory. A worktree agent's `cd <path> && git …` or a chain of git commands is refused.
- **Multi-line content**: code goes into files with Edit or Write. A commit message goes into a file in the agent's scratch folder and is passed with `git commit -F <file>`. Heredocs longer than 15 lines are refused.

## Hotspots

Files nearly every ticket edits, and where merges conflict:

- `crates/core/src/lib.rs`: `Core`, `Ports` and the Operations.
- `crates/core/src/store.rs`: `MIGRATIONS`. Append only; two branches that both append take the next free numbers in merge order, and no migration already on `main` changes.
- `src-tauri/src/lib.rs`: the Tauri command list and the wiring of the Ports.
- `src/operations.ts` and `src/App.tsx`.
- `src/i18n/de.ts` and `src/i18n/en.ts`.
- `GLOSSARY.md`.
- Test fakes: every test file that spells out `Operations` or `Ports` breaks when an Operation or Port is added (#73).
