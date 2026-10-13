# Licht Launcher

An unofficial Minecraft: Java Edition launcher targeting Windows and Linux,
built with Tauri v2, React, TypeScript, and Rust.

**Unofficial project, not affiliated with Mojang Studios or Microsoft.**

## Project status

The window installs and launches. From the instances screen you can install a
version, play or stop it, and read the game log. Sign-in is a Microsoft
account (tokens stay in the OS keyring) or an offline name. Each instance has
its own folder, memory, and Java settings. Fabric, Quilt, Forge, and NeoForge
can be chosen when creating an instance. Modrinth and CurseForge modpacks can
be imported; CurseForge search needs `LICHT_CURSEFORGE_API_KEY`.

Forge/NeoForge use official installer profiles and run client processors during
Install/Repair. Generated libraries are verified before launch; existing
provisional loader profiles are upgraded without changing the instance folder.

`licht` lists Mojang's manifest, installs a version, and launches it with an
offline name. It does not sign in to Microsoft. See the
[roadmap](docs/ROADMAP.md).

What a launch does today:

- Game files come from Mojang and stay in Licht's data directory, not in the
  official `.minecraft` folder. The default Java is Eclipse Temurin. `--java`
  uses another executable. A Java already on the machine is not picked by itself.
- Each instance gets its own folder under `instances/`. LaunchWrapper versions
  (through 1.5.2) keep the game in that folder's `.minecraft` directory.
- Natives are extracted once per version into `natives/<id>`.
- Versions through 1.7.2 get their sounds copied to the names those clients
  open. Newer versions read the hashed asset objects directly.
- The game log is read as UTF-8 on Java 18 and newer, and as the system code
  page on older Java. A bad byte does not kill the process.

Still open: `${auth_session}` is left literal on versions that include it
(through 1.5.2). The live launch checks so far were on Windows.

```sh
cargo run -p licht-core --features cli --bin licht -- versions
cargo run -p licht-core --features cli --bin licht -- install --version 1.20.1
cargo run -p licht-core --features cli --bin licht -- launch --version 1.20.1 --username Player
```

`launch` does not download. `install` does. `--game-dir` replaces the instance
root. `--cache` replaces the data directory. `--java` skips Temurin.

## Prerequisites

- Node.js 24, version 24.15.0 or newer within that major version.
- pnpm 10.33.0, as specified in `package.json`.
- Rust installed through rustup, with `cargo`, `rustc`, and `rustup` on PATH.
  The repository selects Rust 1.99.0, rustfmt, and Clippy through
  `rust-toolchain.toml`.
- [Tauri v2 system prerequisites](https://v2.tauri.app/start/prerequisites/):
  C++ build tools, Windows SDK, and WebView2 on Windows; the required development
  libraries, including WebKitGTK 4.1, on Linux.

## Development

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

The frontend development server is started automatically by the Tauri command.

To build the release executable without an installer:

```sh
pnpm tauri build --ci --no-bundle -- --locked
```

## Quality checks

```sh
pnpm lint
pnpm typecheck
pnpm test
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

The same checks and the Tauri release build run in GitHub Actions on Windows and
Ubuntu. `pnpm lint` checks formatting and imports as well as lint rules.

## Documentation

- [Architecture](docs/ARCHITECTURE.md): layer responsibilities and project boundaries.
- [Roadmap](docs/ROADMAP.md): implementation stages and decisions requiring approval.
- [Compatibility](docs/COMPATIBILITY.md): version differences the launcher has to cover.
- [Versions](docs/core/VERSIONS.md): the Mojang manifest, the four types, and where each version JSON is stored.
- [LaunchWrapper](docs/core/LAUNCHWRAPPER.md): why versions through 1.5.2 use a `.minecraft` folder.
- [Legacy assets](docs/core/LEGACY-ASSETS.md): why versions through 1.7.2 need a sound copy at launch.
- [Runtime](docs/core/RUNTIME.md): Eclipse Temurin, the major each version asks for, and `--java`.
- [Natives](docs/core/NATIVES.md): which native jar is downloaded, and where the binaries are extracted.
- [Forge/NeoForge](docs/core/FORGE.md): official profiles, client processors, readiness, and repair.
- [Log](docs/core/LOG.md): why old versions were failing on a Windows-1252 line, and how the log is decoded.
- [Project rules](AGENTS.md): development conventions.

## License

Licensed under the [MIT License](LICENSE).

Copyright (c) 2026 Mateus Neiva.
