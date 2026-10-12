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
The core also builds the `licht` command-line binary. That binary is the
interface that can list, install, and launch today. The Tauri window does not
call it yet.

The foundation phase is complete: an empty React window, the design system,
TanStack Router, logging, and `CoreError`. Domain work through milestone 1
lives in the core and is reached from `licht`. Tauri commands and events are
phase 5 and are not implemented.

## Communication

The intended flow is:

```text
Frontend -> typed Tauri commands -> licht-core
Frontend <- Tauri events <- adapter handling core progress and status
```

The frontend will use TanStack Query for command loading, caching, and errors.
The adapter will emit progress events with approximately 100 ms throttling.
Shared Rust/TypeScript types must be generated rather than maintained by hand;
the choice between `ts-rs` and `specta`, and the command/event contracts, remains
pending.

## Frontend layout

- `src/routes`: file-based route modules for TanStack Router. Screens stay in `src/features`; a route file only mounts them. The generated tree is `src/routeTree.gen.ts`, and it is committed so typechecking does not depend on a Vite build.
- `src/components/ui`: shared design-system components using theme tokens.
- `src/features`: screens and UI behavior grouped by feature.
- `src/lib`: UI utilities and typed invocation wrappers.

The router is installed. History is hash-based (`/#/`) because the Tauri window does not rewrite paths to `index.html`. Interface copy is English.

TanStack Query, Zustand, and TanStack Virtual are decided and have not been installed yet. Long lists will be virtualized, and screens will reuse the design system. The frontend runs in WebView2 on Windows and WebKitGTK on Linux.

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

## Game directory

Each version keeps its saves next to the data directory, in `instances/<id>`.
Versions that start through LaunchWrapper use `instances/<id>/.minecraft` so the
folder the client reads before `--gameDir` is the instance. The race, the other
launchers, and this choice are recorded in
[LAUNCHWRAPPER.md](core/LAUNCHWRAPPER.md).
