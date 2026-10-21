# Architecture

Licht Launcher is an unofficial Minecraft: Java Edition launcher for Windows and
Linux. It uses Tauri v2, a React + TypeScript + Vite frontend, and a Rust Cargo
workspace. [AGENTS.md](../AGENTS.md) defines the project rules;
[ROADMAP.md](ROADMAP.md) tracks implementation. Version differences are in
[COMPATIBILITY.md](COMPATIBILITY.md).

## Layers

| Location            | Responsibility                                                                           | Boundary                                           |
| ------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `crates/licht-core` | Minecraft manifests, rules, downloads, Java runtimes, game execution, and authentication | A standalone Rust library with no Tauri dependency. Internally grouped as `meta/`, `transfer/`, `game/`, `instance/`, `runtime/`, `store/`, plus `auth/`, `loaders/`, and `modpacks/` |
| `src-tauri`         | Application startup, logging configuration, thin commands, and frontend events           | Delegates game behavior to the core                |
| `src`               | Screens, UI state, and calls to the Tauri adapter                                        | Contains no Minecraft domain logic                 |

The Cargo workspace contains `licht-core` and the `licht-launcher` application
package. The application depends on the core through a local path dependency.
The core also builds the `licht` command-line binary. Both the CLI and the
Tauri adapter call the same core services; the frontend uses typed commands for
launcher actions and listens for progress and status events.

## Communication

The implemented flow is:

```text
Frontend -> typed Tauri commands -> licht-core
Frontend <- Tauri events <- adapter handling core progress and status
```

TanStack Query provides command loading, caching, mutations, and error states.
The adapter emits progress events with approximately 100 ms throttling.
Shared Rust/TypeScript types are generated with `ts-rs`; generated files live
under `src/lib/generated`.

## Frontend layout

- `src/routes`: file-based route modules for TanStack Router. Screens stay in `src/features`; a route file only mounts them. The generated tree is `src/routeTree.gen.ts`, and it is committed so typechecking does not depend on a Vite build.
- `src/components/ui`: shared design-system components using theme tokens.
- `src/features`: screens and UI behavior grouped by feature.
- `src/lib`: UI utilities and typed invocation wrappers.

The router is installed. History is hash-based (`/#/`) because the Tauri window does not rewrite paths to `index.html`. Interface copy is English.

TanStack Query is installed and used for Tauri command state. TanStack Virtual
is installed but not wired to the current short lists. Zustand is not installed;
feature state currently uses React hooks and the existing query/cache layer.
The frontend runs in WebView2 on Windows and WebKitGTK on Linux.

## Errors and logging

The core exports `CoreError`, derived with `thiserror`, and a `Result<T>` alias.
Its initial `Io` variant preserves the original I/O error as its source.
Application startup uses `anyhow` to add context at the boundary; errors are
propagated with `?` rather than unwrapped or silently discarded.

Logging uses `tracing`. The executable configures a `tracing-subscriber` before
starting Tauri, writes structured text to stderr, and uses INFO when `RUST_LOG`
is absent or empty. Invalid filters return an error. Logs must not contain
tokens, secrets, or personal data.

## Testing

The core is testable independently with `cargo test -p licht-core`. Integration
tests under `crates/licht-core/tests/` mirror the source domains (`meta`,
`transfer`, `game`, `instance`, `runtime`, `store`); fixtures stay in
`tests/fixtures/`. Domain tests use saved Mojang JSON and must not depend on
network access. Frontend tests use Vitest, Testing Library, and jsdom.

CI runs Biome checks, TypeScript typechecking, frontend tests, Rust formatting,
Clippy, Rust tests, and a Tauri release build on Windows and Ubuntu. Automated
builds and tests are separate from manual graphical validation.

## Data on disk

The data directory comes from the `directories` crate
(`app.licht` / `Licht` / `Licht Launcher`). On Windows that is under
`%APPDATA%`. It holds `assets/indexes`, `assets/objects`, `libraries`,
`natives/<version id>`, `runtime`, and `versions/<id>`. `assets/virtual/<index id>`
is created when a `virtual` index is launched. `--cache` replaces this root.

`instances/<id>` is the sibling of the data directory, not a folder inside it.

### Dedicated SQLite service

`store/service.rs` owns an asynchronous dispatcher and a persistent SQLx SQLite
connection. SQLx executes SQLite's blocking API on its own background worker;
the launcher does not add another blocking SQL thread. Startup opens and migrates
the connection before returning the service owner. Cloneable `DatabaseClient`
handles submit typed requests over a FIFO queue with capacity for 64 pending
operations and await individual replies. A full queue applies asynchronous
backpressure rather than blocking a Tokio executor thread.

The application routes instance profiles, last-played metadata, download jobs,
artifact associations, startup recovery, and remembered account UUIDs through this
service. Both profile timestamps and global last-played metadata use SQLite.
Frontend command names, inputs, and responses
remain the same. Rust persistence and download-manager methods that access the
database are asynchronous and callers explicitly await them.

`store/repositories/` contains concrete `MetadataRepository`,
`DownloadJobRepository`, `DownloadArtifactRepository`, and `InstanceRepository` implementations. They
own parameterized SQL, row mapping, database validation, and transactions. The
download manager coordinates installation state and emits events after persistence
completes. Filesystem operations and destination locks stay outside repositories.
Tests exercise repositories through the public service and standalone persistence
APIs using real temporary SQLite databases; query macros requiring a build-time
database or checked-in query metadata are not used.

An accepted request runs even if its caller stops awaiting its reply. SQL failures
are returned to the caller; failures without a remaining caller are logged without
query parameters or stored values. During shutdown the adapter first suspends
downloads, then stops database admission, drains accepted requests, and awaits the
dispatcher and SQLx connection closure. Dropping the service owner also signals
the dispatcher to stop and drain, but explicit shutdown is required to await
completion and receive close errors.

The database file, schema version 7, legacy migration paths, stored enum names,
indices, and five-second busy timeout remain compatible with existing installs.
Migration checks avoid a writer transaction for an up-to-date schema and recheck
the version inside the migration transaction. Standalone persistence functions
also use SQLx repositories and explicitly close their short-lived connections.
Account identifiers retain the existing metadata format and updates are
transactional. Account tokens remain in the OS keyring. There is no rusqlite
dependency or second persistence implementation.

### Instance profiles

The `instances` table stores profiles keyed by `(instances_directory,
instance_folder)`. Profile settings have dedicated columns; JVM argument lists
and the optional installation selection/state use JSON columns. Reads validate
stored profiles with the same rules as writes. Instance operations are asynchronous
and share an asynchronous mutation lock, preventing settings edits and background
preparation from overwriting each other's changes. Deleting a profile and its
modpack index is a single database transaction after filesystem cleanup succeeds.

New instances never write `instance.json`. Existing profile files are neither
imported nor removed; folders without a SQLite profile are not listed or
automatically registered. A missing registered instance directory is omitted from
the list. Duplication copies game files and creates a separate database profile,
without copying obsolete root profile files or the installation workspace. The
CLI can still launch an unregistered game directory using global settings.

## Game directory

Each version keeps its saves next to the data directory, in `instances/<id>`.
Versions that start through LaunchWrapper use `instances/<id>/.minecraft` so the
folder the client reads before `--gameDir` is the instance. The race, the other
launchers, and this choice are recorded in
[LAUNCHWRAPPER.md](core/LAUNCHWRAPPER.md).
