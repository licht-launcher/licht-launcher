# Contributing to Licht Launcher

Thank you for your interest in Licht Launcher. The project currently has one
maintainer, Mateus Neiva ([@mateusneiva](https://github.com/mateusneiva)), who
handles development, contribution review, and release decisions.

## Proposing a change

Read [AGENTS.md](AGENTS.md), the [roadmap](docs/ROADMAP.md), and the
[architecture](docs/ARCHITECTURE.md) before starting. Open an
[issue](https://github.com/licht-launcher/licht-launcher/issues) to describe a bug
or discuss a feature. Include reproduction steps, OS, launcher version, and
the relevant game/loader version when applicable. Redact personal information
from logs and screenshots.

For non-trivial changes, agree on a short plan and scope with the maintainer
before implementation. Work on one item at a time. Dependencies, architecture,
authentication/security decisions, shared Rust/frontend APIs, and roadmap
items marked for approval require discussion first.

For suspected vulnerabilities or private reports, email
[mateus.fneiva@gmail.com](mailto:mateus.fneiva@gmail.com) instead of publishing
exploitable details, credentials, or personal data in an issue.

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

### CLI

The `licht` CLI lists Mojang's manifest, installs a version, and launches a
cached version with an offline name:

```sh
cargo run -p licht-core --features cli --bin licht -- versions
cargo run -p licht-core --features cli --bin licht -- install --version 1.20.1
cargo run -p licht-core --features cli --bin licht -- launch --version 1.20.1 --username Player
```

`launch` does not download. `install` does. `--game-dir` replaces the instance
root, `--cache` replaces the data directory, and `--java` selects another Java
executable. See the [version](docs/core/VERSIONS.md), [runtime](docs/core/RUNTIME.md),
and [compatibility](docs/COMPATIBILITY.md) documentation for details.

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
  requests, local storage, or data handling in [PRIVACY.md](PRIVACY.md).

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

## Pull requests and review

Use a focused branch such as `feat/...`, `fix/...`, or `chore/...` and small
[Conventional Commits](https://www.conventionalcommits.org/).
Submit a PR against `main` with:

- what changed and why;
- how to test, including commands actually run and their results;
- decisions made and points needing maintainer review;
- a **Rust concepts used** section explaining new Rust concepts in plain
  language, or stating that no Rust code changed.

External contributions require maintainer review before merging. Changes to
CI, build scripts, dependencies, downloads, authentication, and signing deserve
particular attention. Respect copyright and licenses; original contributions
are submitted under the project's [MIT License](LICENSE), with existing
third-party notices and licenses preserved.

## Releases and signing responsibilities

The [Code signing policy](docs/CODE_SIGNING.md) identifies the current
author/committer, reviewer, and release-signing approver. The sole maintainer
currently fills all three roles; there is no separate independent reviewer.
Contributing a PR does not grant repository write access or signing access.

Before enabling a future SignPath integration, repository/signing account
holders must meet its MFA requirements. Each SignPath signing request must be
manually approved by the designated approver after reviewing the source commit,
build provenance, successful checks, and intended artifacts. Signing access
must never be used for unrelated projects or downloaded third-party binaries.
SignPath artifact restrictions must enforce the Licht Launcher product name
and matching product versions. Release/download pages for SignPath-signed builds
must link the Code signing policy and identify their actual signing status.
The current Tauri updater-signature workflow is documented in
[docs/UPDATES.md](docs/UPDATES.md); SignPath has not yet been applied for or
integrated.

## Contact and project policies

Be respectful, constructive, and specific in discussions. For project questions,
contact the maintainer through GitHub or
[mateus.fneiva@gmail.com](mailto:mateus.fneiva@gmail.com). See the
[Terms of Use](TERMS.md) and [Privacy Policy](PRIVACY.md) for the project's
user-facing policies.
