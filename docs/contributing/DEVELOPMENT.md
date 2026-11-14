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
pnpm dev
```

The frontend development server starts automatically with `pnpm dev`.
For a frontend-only development server, run `pnpm dev:web`.

To build the release executable without an installer:

```sh
pnpm tauri build --ci --no-bundle -- --locked
```

Use pnpm and the committed lockfiles. Discuss new dependencies before adding
them. Use your own accounts and authorized test data for manual authentication
tests. Never commit credentials, API keys, signing keys, or account captures.

## CLI

The `licht-cli` lists Mojang's manifest, installs a version, lists saved
instances, and launches a cached version or instance with an offline name:

```sh
cargo run -p licht-cli -- versions
cargo run -p licht-cli -- install --version 1.20.1
cargo run -p licht-cli -- instances list
cargo run -p licht-cli -- launch --version 1.20.1 --username Player
cargo run -p licht-cli -- launch --instance Survival --username Player
```

`launch` does not download. `install` does. `--instance` accepts a folder or a
unique display name and cannot be combined with `--version` or `--game-dir`.
`--game-dir` replaces the instance root for `--version` launches, `--cache`
replaces the data directory, and `--java` selects another Java executable. See
the [runtime](../internals/RUNTIME.md), [storage](../internals/STORAGE.md), and
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
pnpm check
```

`pnpm check` runs the frontend checks (`pnpm check:web`: lint, typecheck,
tests) and the Rust checks (`pnpm check:rust`: `cargo fmt --check`, Clippy,
tests). Run either half on its own when you only touched one side.

For build/packaging changes, also run the relevant build described in the
development environment section above. Report checks you could not run and actual
failures; do not disable lint rules or weaken tests to obtain a passing result.

### End-to-end tests

`e2e/` drives the real application with WebdriverIO and
[`@wdio/tauri-service`](https://webdriver.io/docs/desktop-testing/tauri/) through
`tauri-driver` (Windows and Linux only). The `E2E` workflow runs them on pull
requests that touch the app.

One-time setup:

- `cargo install tauri-driver --version 2.1.0 --locked` (or point
  `TAURI_DRIVER_PATH` at an existing `tauri-driver` binary).
- Linux: install the package that provides `WebKitWebDriver`
  (`webkitgtk-webdriver` on Ubuntu 26.04+, `webkit2gtk-driver` on older
  Debian/Ubuntu); use `xvfb-run` when there is no display.
- Windows: nothing else; the service downloads the Microsoft Edge Driver that
  matches the installed WebView2 into a new `%TEMP%\msedgedriver\<major>-<random>`
  folder on every run (about 26 MB). To skip that, put a matching
  `msedgedriver.exe` on `PATH`; the service reuses it when the major version matches.

Run:

```sh
pnpm build:e2e
pnpm test:e2e
```

`pnpm build:e2e` builds a debug binary with the `e2e` Cargo feature and
`src-tauri/tauri.e2e.conf.json`, which add the `tauri-plugin-wdio` bridge that
the service needs. Release builds never enable that feature. Each run starts
the app with a temporary `LICHT_DATA_DIR`, so your own launcher data is never
touched, and with `LICHT_SKIP_UPDATE_CHECK=1`, which debug builds honor to skip
background update checks. Keep end-to-end tests off flows that need the
network or a Microsoft account (downloads, sign-in, launching the game).
