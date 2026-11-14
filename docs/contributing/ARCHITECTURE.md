# Architecture

Licht Launcher is an unofficial Minecraft: Java Edition launcher for Windows and
Linux, built with Tauri v2, React, TypeScript, Vite, and a Rust Cargo workspace.
[AGENTS.md](../../AGENTS.md) defines project rules;
[ROADMAP.md](../ROADMAP.md) tracks pending work.

## Layers

| Location | Responsibility | Boundary |
| --- | --- | --- |
| `crates/licht-core` | Manifests, downloads, Java, game execution, instances, authentication, loaders, and modpacks | Standalone Rust library without Tauri dependencies |
| `crates/licht-cli` | Command-line parsing, terminal output, and orchestration | Delegates domain behavior to `licht-core` |
| `src-tauri` | Startup, logging configuration, typed commands, and events | Delegates domain behavior to the core |
| `src` | Screens, UI state, and calls to the Tauri adapter | No Minecraft domain logic |

The workspace contains the `licht-core` library, the `licht-cli` command-line
application, and the `licht-launcher` desktop application. Both applications call
the same core services. Core modules include `meta`, `transfer`, `game`, `instance`,
`runtime`, `store`, `auth`, `loaders`, and `modpacks`.

## Communication

```text
Frontend -> typed Tauri commands -> licht-core
Frontend <- Tauri events <- adapter handling core progress and status
```

TanStack Query handles command loading, caching, mutations, and errors. Progress
events are throttled to approximately 100 ms. Shared types are generated with
`ts-rs` into `src/lib/generated`; they are not maintained separately in TypeScript.

## Frontend layout

- `src/routes`: TanStack Router file-based routes that mount feature screens.
- `src/features`: screens and feature-specific UI behavior.
- `src/components/ui`: shared design-system components using theme tokens.
- `src/api`: bridge to the Rust backend and Tauri: typed invocation wrappers,
  Rust payload parsing, app events, and window/platform helpers.
- `src/lib`: generic helpers (`cn`, error messages, formatting) and the
  `ts-rs` types in `src/lib/generated`.

Each component lives in its own folder named after it, for example
`src/components/ui/button/`:

- `button.tsx`: markup and behavior;
- `button.styles.ts`: its Tailwind classes as an `as const` object (and `cva`
  variants), omitted when the component has no classes;
- `button.test.tsx`: its tests;
- `index.ts`: `export * from "./button";`, so imports stay
  `@/components/ui/button`.

The shadcn CLI writes flat files (`src/components/ui/<name>.tsx`). After
`pnpm dlx shadcn add <name>`, move the file into `src/components/ui/<name>/`,
move its classes into `<name>.styles.ts`, and add `index.ts` and a test.

`.vscode/settings.json` configures Tailwind CSS IntelliSense to recognize
classes inside `cn`/`cva` calls and `*Styles = { ... } as const` objects.

The generated `src/routeTree.gen.ts` is committed so typechecking does not depend
on a Vite build. Hash history (`/#/`) avoids requiring the Tauri window to rewrite
paths to `index.html`. Interface copy is English. Feature state uses React hooks
and the query/cache layer. The UI runs in WebView2 on Windows and WebKitGTK on Linux.

## Errors and logging

The core exports `CoreError`, derived with `thiserror`, and a `Result<T>` alias.
I/O errors preserve their source. Application startup uses `anyhow` to add context
at the boundary; errors propagate with `?`.

Logging uses `tracing` and `tracing-subscriber`, with structured text on stderr.
`RUST_LOG` selects filtering; absent or empty values default to INFO, and invalid
filters return an error. Logs must not contain tokens, secrets, or personal data.

## Related internals

- [Storage](../internals/STORAGE.md): profile/content directories, relocation, SQLite service,
  and instance persistence.
- [Runtime](../internals/RUNTIME.md): Java selection, installation, and discovery.
- [Loaders](../internals/LOADERS.md): loader profiles and Forge/NeoForge processors.
- [Compatibility](../internals/COMPATIBILITY.md): manifest formats, legacy execution, native
  libraries, assets, logs, and known gaps.
- [Integrations](../integrations/MODRINTH.md): Modrinth, alongside
  [CurseForge](../integrations/CURSEFORGE.md) and
  [Microsoft authentication](../integrations/MICROSOFT.md).

## Testing

The core is independently testable with `cargo test -p licht-core`. Integration
tests under `crates/licht-core/tests/` mirror source domains; fixtures live under
`tests/fixtures/`. Tests use saved JSON and temporary databases without network
access. Frontend tests use Vitest, Testing Library, and jsdom.

CI checks frontend lint, types, and tests, Rust formatting, Clippy, and tests, and
builds Tauri on Windows and Ubuntu. Build/test coverage is distinct from manual
graphical validation. See [Development](DEVELOPMENT.md#implementation-and-testing)
for commands.

## References

- [Mojang version manifest](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json).
- [minecraft.wiki](https://minecraft.wiki/): JSON formats and authentication.
- Reference implementations: Prism Launcher, HMCL, portablemc, minecraft-launcher-lib.
- [Tauri v2 documentation](https://v2.tauri.app).
