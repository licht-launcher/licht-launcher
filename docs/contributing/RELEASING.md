# Releasing Licht Launcher

## Codenames

Each `0.x` release gets the name of a flower, in German, in this order:

| Version | Codename  | Meaning    |
| ------- | --------- | ---------- |
| 0.1     | Lilie     | lily       |
| 0.2     | Iris      | iris       |
| 0.3     | Rose      | rose       |
| 0.4     | Immergrün | periwinkle |
| 0.5     | Anemone   | anemone    |
| 0.6     | Narzisse  | daffodil   |
| 0.7     | Edelweiß  | edelweiss  |

From 0.8 on, the list continues (to be defined together with the project owner).

## Where the codename appears

- GitHub Release title.
- Installer file name (e.g. `Licht-Launcher-0.1.0-Lilie`).
- "About" screen, next to the version.

## Rules

- Commits follow Conventional Commits.
- Releases are produced by CI from a tag (see [Roadmap](../ROADMAP.md), item 10.3).
- Keep the notice "not affiliated with Mojang Studios or Microsoft" in the README and in the release notes.

## Updates

Licht uses the Tauri v2 updater with GitHub Releases. Release notes belong in each
GitHub Release. The workflow publishes signed Windows NSIS and Linux AppImage
update artifacts, plus the manifest read by the updater:

```text
https://github.com/licht-launcher/licht-launcher/releases/latest/download/latest.json
```

Updater signatures verify update packages. They are separate from Windows
publisher signing; status and responsibilities live in the
[code signing policy](CODE_SIGNING.md).

### Signing key

The public key is committed in `src-tauri/tauri.conf.json`. Configure these
repository secrets before publishing:

- `TAURI_SIGNING_PRIVATE_KEY`: contents of the key generated with
  `pnpm tauri signer generate`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: empty for the current key, or the password
  chosen for a protected key.

Never commit or log the private key. Installed applications accept updates signed
for their shipped public key. Key rotation requires an update signed with the old
key that carries the new public key. Losing the private key prevents further
signed updates for those installs; users must reinstall manually.

### User flow

- **Automatic updates** checks after the main window opens and every 30 minutes
  while no verified package is waiting to be applied. Available updates download
  in the background; installation always requires **Restart to update**.
- Turning automatic updates off cancels automatic checks/downloads. Manual
  checks and downloads remain available, and verified cached packages are kept.
- Existing preferences migrate conservatively: automatic updates are enabled
  only when both automatic checking and automatic startup installation were on.
- Packages and metadata live in the settings directory's `cache/app-updates`.
  Cache reads verify SHA1 and the Minisign signature using the configured public
  key, including the updater's configured signed-version policy.
- After reopening, a cached package needs an online manifest check before it can
  be installed. Matching versions and signatures reuse the package instead of
  downloading it again. Packages for an already installed version are removed.
- Settings and the title bar provide explicit update actions. Before installing,
  downloads are suspended. Windows drains launcher services in the updater's
  exit callback; Linux uses the normal restart/shutdown path after installation.
- **Later** dismisses the ready notification without deleting the package or
  changing preferences. Closing/reopening the app never installs automatically.
- Failures leave the installed launcher available and expose a retry in Updates.
  An unsuccessful installation attempt still requires another explicit action.
- The startup installer remains available for explicit requests saved by older
  launcher versions; it no longer checks/downloads automatically during startup.
