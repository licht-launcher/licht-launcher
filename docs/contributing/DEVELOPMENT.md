# Development

Start with [Contributing](../../CONTRIBUTING.md) for proposing changes and opening
PRs. [AGENTS.md](../../AGENTS.md) defines project rules, and
[Architecture](ARCHITECTURE.md) explains the layers and their boundaries.

## Development environment

The desktop application uses Tauri v2, React, TypeScript, and Rust. Use the
versions pinned by the repository:

- Node.js 24, version 24.15.0 or newer within that major version.
- pnpm 10.33.0, as specified in `package.json`.
- Rust 1.99.0, selected by `rust-toolchain.toml`, with Cargo, rustfmt, and Clippy.
- [Tauri v2 system prerequisites](https://v2.tauri.app/start/prerequisites/):
  C++ build tools, Windows SDK, and WebView2 on Windows; the required development
  libraries, including WebKitGTK 4.1, on Linux.

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

The frontend development server starts automatically with `pnpm tauri dev`.
For a frontend-only development server, run `pnpm dev`.

To build the release executable without an installer:

```sh
pnpm tauri build --ci --no-bundle -- --locked
```

Use pnpm and the committed lockfiles. Discuss new dependencies before adding
them. Use your own accounts and authorized test data for manual authentication
tests. Never commit credentials, API keys, signing keys, or account captures.

## CLI

The `licht` CLI lists Mojang's manifest, installs a version, and launches a
cached version with an offline name:

```sh
cargo run -p licht-core --features cli --bin licht -- versions
cargo run -p licht-core --features cli --bin licht -- install --version 1.20.1
cargo run -p licht-core --features cli --bin licht -- launch --version 1.20.1 --username Player
```

`launch` does not download. `install` does. `--game-dir` replaces the instance
root, `--cache` replaces the data directory, and `--java` selects another Java
executable. See the [runtime](../internals/RUNTIME.md),
[storage](../internals/STORAGE.md), and
[compatibility](../internals/COMPATIBILITY.md) documentation for details.

## Implementation and testing

- Follow the surrounding style and keep changes focused. Avoid unrelated
  refactoring or speculative abstractions.
- Write code, comments, commit messages, and PR descriptions in English.
  Interface copy is also English.
- Keep Minecraft logic in `crates/licht-core`, Tauri commands/events in the
  thin `src-tauri` layer, and UI logic in `src`.
- Use strict TypeScript and existing design-system components/theme tokens.
  Keep the UI keyboard-accessible and compatible with Windows WebView2 and
  Linux WebKitGTK.
- Generate shared Rust/TypeScript types through the existing `ts-rs` process;
  do not duplicate them by hand.
- Reproduce bugs with a failing behavioral test before fixing them. Cover new
  core logic and meaningful UI interactions with automated tests. Automated
  tests must not depend on the network or a real Microsoft account.
- Reuse existing fixtures. Include third-party test material only when its
  redistribution is permitted, and document its source. Do not submit
  proprietary game binaries, textures, sounds, or private account responses.
- Keep tokens in the OS keyring and out of logs. Document changes to external
  requests, local storage, or data handling in [PRIVACY.md](../../PRIVACY.md).

Run these checks before submitting a PR:

```sh
pnpm lint
pnpm typecheck
pnpm test
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

For build/packaging changes, also run the relevant build described in the
development environment section above. Report checks you could not run and actual
failures; do not disable lint rules or weaken tests to obtain a passing result.
