# Contributing to Licht Launcher

Thank you for your interest in Licht Launcher. The project currently has one
maintainer, Mateus Neiva ([@mateusneiva](https://github.com/mateusneiva)), who
handles development, contribution review, and release decisions.

## Proposing a change

Read [AGENTS.md](AGENTS.md), the [roadmap](docs/ROADMAP.md), and the
[architecture](docs/ARCHITECTURE.md) before starting. Open an
[issue](https://github.com/mateusneiva/licht-launcher/issues) to describe a bug
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

Use the versions pinned by the repository: Node.js 24 within the supported
range, pnpm 10.33.0, and Rust 1.99.0 through rustup. Install the
[Tauri v2 system prerequisites](https://v2.tauri.app/start/prerequisites/)
for your OS. See the [README](README.md#prerequisites) for details.

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

Use pnpm and the committed lockfiles. Discuss new dependencies before adding
them. Use your own accounts and authorized test data for manual authentication
tests. Never commit credentials, API keys, signing keys, or account captures.

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
[README](README.md#development). Report checks you could not run and actual
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

The [Code signing policy](README.md#code-signing-policy) identifies the current
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
