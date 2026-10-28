# Contributing to Licht Launcher

Thank you for your interest in Licht Launcher. The project currently has one
maintainer, Mateus Neiva ([@mateusneiva](https://github.com/mateusneiva)), who
handles development, contribution review, and release decisions.

## Proposing a change

Read [AGENTS.md](AGENTS.md), the [roadmap](docs/ROADMAP.md), and the
[architecture](docs/contributing/ARCHITECTURE.md) before starting. Open an
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

## Development and testing

Follow the [development guide](docs/contributing/DEVELOPMENT.md) for system
prerequisites, local setup, CLI usage, implementation guidance, and required
checks. Run the applicable checks before submitting a PR and report their results.

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

The [Code signing policy](docs/contributing/CODE_SIGNING.md) defines signing status,
responsibilities, and requirements for a future SignPath integration.
Contributing a PR does not grant repository write access or signing access.
Version conventions and the current updater-signature workflow are documented
in [Releasing](docs/contributing/RELEASING.md).

## Contact and project policies

Be respectful, constructive, and specific in discussions. For project questions,
contact the maintainer through GitHub or
[mateus.fneiva@gmail.com](mailto:mateus.fneiva@gmail.com). See the
[Terms of Use](TERMS.md) and [Privacy Policy](PRIVACY.md) for the project's
user-facing policies.
