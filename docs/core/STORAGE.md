# Storage directories

## Profile directory

The profile contains `settings.json`, `licht.db` (and SQLite WAL files), startup
requests, and the `content-directory.json` move checkpoint. It normally lives in
the system data directory under `Licht Launcher`.

Set **`LICHT_DATA_DIR`** before starting the launcher to use a different profile.
Its value must be an absolute path without `..` components. An empty or invalid
value is an error. The launcher uses the specified directory directly, without
adding another `Licht Launcher` subdirectory.

An overridden profile does not import settings, databases, or content from the
normal or legacy profile. A new profile starts with defaults. Its own saved
application-directory setting still applies. This allows an isolated test run to
use an empty temporary directory.

PowerShell:

```powershell
$env:LICHT_DATA_DIR = 'D:\LichtTestProfile'
& 'C:\path\to\licht.exe'
```

Linux:

```sh
LICHT_DATA_DIR=/absolute/path/to/test-profile /path/to/licht
```

## Application directory setting

**Settings → Downloads → Application directory** chooses the content location.
The default layout is:

```text
<profile>/
  settings.json
  licht.db
  content-directory.json
  data/       # game files and managed Java runtimes
  instances/  # instance files, mods, saves, and other user content
  cache/      # content caches and profile operational files
```

For a custom choice, content lives under `<chosen>/Licht Launcher/`, with `data`,
`instances`, and the content caches (`cache/icons` and `cache/content-metadata`).
Profile operational files, player-head caches, settings, and the database remain
in the profile directory. Changing the executable's installation location does
not change either directory.

The selected destination is saved immediately, but the running process continues
using its original directories. On the next startup, before services start, the
launcher copies and SHA1-verifies the content, remaps persisted instance, Java,
native-cache, modpack, and download paths, and then removes verified source files.
Clearing the setting moves content back to the default location.

Different files at an existing destination cause an error; they are not
overwritten. The source is retained until its destination has been verified.
Directories must not overlap, and linked directories/files are not followed.
The previous content directory must be available when copying begins.

The checkpoint allows copying, database remapping, and cleanup to be retried
after an interrupted startup. Resolve any reported destination conflict before
retrying. Once copying has completed, finish the pending move before selecting
another destination. The first
startup of an existing installation records its current content directory without
moving it.

## Verification

Offline relocation tests use temporary directories:

```sh
cargo test -q -p licht-core --lib store::layout::relocation
cargo test -q -p licht-core --lib store::cache::override_tests
pnpm vitest run src/features/settings/sections/downloads-section.test.tsx
```
