# Licht Launcher

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Licht Launcher icon" width="128">
</p>

<p align="center">
  <strong>A light, cross-platform launcher for Minecraft: Java Edition.</strong><br>
  Built with Tauri v2, React, TypeScript, and Rust for Windows and Linux.
</p>

<p align="center">
  <a href="https://github.com/mateusneiva/licht-launcher/actions/workflows/ci.yml"><img src="https://github.com/mateusneiva/licht-launcher/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="https://github.com/mateusneiva/licht-launcher/releases/latest"><img src="https://img.shields.io/github/v/release/mateusneiva/licht-launcher?label=latest%20release" alt="Latest release"></a>
  <a href="https://github.com/mateusneiva/licht-launcher/releases"><img src="https://img.shields.io/github/downloads/mateusneiva/licht-launcher/total?label=downloads" alt="Downloads"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/mateusneiva/licht-launcher" alt="MIT license"></a>
</p>

**Current release: `0.2.2 · Iris`**

**Unofficial project, not affiliated with Mojang Studios or Microsoft.**

## Download

Download the latest build from the
[GitHub Releases page](https://github.com/mateusneiva/licht-launcher/releases/latest).

| Platform | Recommended download |
| --- | --- |
| Windows x64 | NSIS installer (`.exe`) |
| Linux x64 | AppImage |
| Linux distributions with Debian packages | `.deb` |

Portable binaries are also published for Windows and Linux. The updater uses
GitHub Releases as its source and currently supports signed Windows NSIS and
Linux AppImage updates.

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
offline name. The desktop application also supports Microsoft account flows,
instances, mod loaders, modpack imports, live logs, and update checks. See the
[roadmap](docs/ROADMAP.md) for completed work and remaining scope.

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

For a frontend-only development server:

```sh
pnpm dev
```

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

Release signing and the automatic update flow are documented in
[docs/UPDATES.md](docs/UPDATES.md). The update private key must remain outside
the repository and be configured as a GitHub Actions secret.

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

## Contributing

Read [AGENTS.md](AGENTS.md) and [docs/ROADMAP.md](docs/ROADMAP.md) before
starting work. Changes should be focused, tested, and submitted through a pull
request. Do not commit game files, signing keys, API keys, or other secrets.

## License

Licensed under the [MIT License](LICENSE).

Copyright (c) 2026 Mateus Neiva.
