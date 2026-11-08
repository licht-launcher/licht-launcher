# Security Policy

## Supported versions

Only the latest release of Licht Launcher receives security fixes. Please
update to the latest version before reporting, and confirm the problem still
happens there.

## Reporting a vulnerability

Please report suspected vulnerabilities privately instead of opening a public
issue.

Email [mateus.fneiva@gmail.com](mailto:mateus.fneiva@gmail.com) with the
subject `Licht Launcher security report` and include:

- the launcher version and operating system;
- a description of the problem and its impact;
- steps to reproduce, or a proof of concept;
- any logs or screenshots, with tokens, account details, and personal data
  removed.

Do not include real Microsoft or Minecraft credentials, access tokens, or
refresh tokens in a report.

## What to expect

Licht Launcher is maintained by one person, so these are good-faith targets, not
guarantees:

- acknowledgement of your report within 7 days;
- an initial assessment and, if confirmed, a plan for a fix within 30 days;
- a coordinated disclosure: please give me a reasonable time to ship a fix
  before sharing details publicly, up to 90 days from the report.

Reporters are credited in the release notes unless they prefer to stay
anonymous.

## Scope

In scope:

- the launcher application and its Rust core;
- authentication and token storage;
- downloads, file verification, and extraction of game files, mods, and
  content;
- the update mechanism and release signing.

Out of scope:

- vulnerabilities in Minecraft, mods, modpacks, mod loaders, or other
  third-party content that the launcher only downloads or runs;
- vulnerabilities in Microsoft, Mojang, or other third-party services;
- issues that require physical access to an unlocked device or an already
  compromised operating system;
- social engineering, spam, or denial-of-service against third-party services.

For problems that are not security-sensitive, [open an
issue](https://github.com/licht-launcher/licht-launcher/issues) instead. The
[Privacy Policy](PRIVACY.md) describes what data the launcher handles.
