# Updates

Licht Launcher uses the Tauri updater with GitHub Releases as its update
source.

## Release artifacts

The release workflow creates signed NSIS artifacts for Windows and signed
AppImage artifacts for Linux. It publishes `latest.json` alongside the
installers. The updater reads that manifest from:

```text
https://github.com/mateusneiva/licht-launcher/releases/latest/download/latest.json
```

Only the public signing key is committed in
`src-tauri/tauri.conf.json`. The private key must never be committed or
included in logs.

## GitHub Actions secrets

Configure these repository secrets before publishing a release:

- `TAURI_SIGNING_PRIVATE_KEY`: contents of the private key generated with
  `pnpm tauri signer generate`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: empty for the current key, or the
  password chosen when generating a protected key.

If the private key is lost, future updates cannot be signed with the public key
already shipped in the application. Generate a new key only as a deliberate
key-rotation operation, because it requires shipping a new application through
the existing update channel first.

## User flow

On every application start, the launcher checks for an update. A network or
server failure is reported as a toast and does not prevent the launcher from
opening.

When an update is found, the user can install it immediately or choose
**Later**. Choosing **Later** stores the update version locally. On the next
start, that same update is downloaded, installed, and the launcher is
restarted automatically.
