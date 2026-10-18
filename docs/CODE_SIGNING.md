# Code signing policy

## Current signing status

**SignPath status: application not yet submitted; SignPath signing is not yet
integrated.** The project currently publishes Tauri updater signatures. These
allow the updater to verify update packages and are separate from Windows
Authenticode signatures identifying a publisher. No SignPath approval or
certificate sponsorship is claimed for current releases.

## Responsibilities

Licht Launcher is currently a single-maintainer project:

| Role | Responsible person |
| --- | --- |
| Author / committer | [Mateus Neiva (@mateusneiva)](https://github.com/mateusneiva) |
| Reviewer of external contributions | [Mateus Neiva (@mateusneiva)](https://github.com/mateusneiva) |
| Release and code-signing approver | [Mateus Neiva (@mateusneiva)](https://github.com/mateusneiva) |

These roles are held by the same person; the project does not currently have
an independent reviewer.

## Policy for a future SignPath integration

- Repository and signing account access must use multi-factor authentication.
- External contributions must be reviewed before merging. CI/build scripts,
  dependencies, and artifact changes are included in that review.
- Each SignPath signing request must be manually approved by the listed
  approver after checking the source commit, CI results, artifact provenance,
  and product/version metadata. The current release workflow does not yet
  implement that SignPath approval step.
- Signing is limited to Licht Launcher's own artifacts built from this
  repository. Downloaded Minecraft files, Java runtimes, mod loaders, and mods
  must not be submitted as the project's own binaries for signing.

If SignPath accepts the project and signing is integrated, the attribution for
artifacts signed through that program will be:

> Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

Data handling, automatic network requests, available controls, and third-party
services are described in the [Privacy Policy](../PRIVACY.md).
SignPath's [conditions](https://signpath.org/terms) also require appropriate
installation-time privacy disclosure and controls for covered data-transfer
features; publication of these documents alone does not add those
installer/application controls.

See [contribution responsibilities](../CONTRIBUTING.md#releases-and-signing-responsibilities)
for the review process and [Updates](UPDATES.md) for the current updater-signature
workflow.
