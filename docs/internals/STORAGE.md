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

Content relocation described in this section is pending in
[PR #168](https://github.com/licht-launcher/licht-launcher/pull/168); it is not yet
available on `main`.

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

## Persistence

The profile's `licht.db` stores instance profiles, last-played metadata, download
jobs, artifact associations, recovery state, and remembered account UUIDs. Tokens
remain in the OS keyring; see [Microsoft authentication](../integrations/MICROSOFT.md).

### Database service

`store/service.rs` owns an asynchronous dispatcher and a persistent SQLx SQLite
connection. SQLx runs SQLite's blocking API on its own worker. Cloneable
`DatabaseClient` handles submit typed requests through a FIFO queue with capacity
for 64 pending operations; a full queue applies asynchronous backpressure.

`store/repositories/` owns parameterized SQL, row mapping, validation, and
transactions. Filesystem operations and destination locks stay outside the
repositories. Persistence APIs are asynchronous; callers await their results.
Tests use real temporary SQLite databases without build-time database metadata.

Accepted requests finish even if their caller stops awaiting them. SQL errors
return to the caller; errors without a remaining caller are logged without query
parameters or stored values. Shutdown suspends downloads, stops admission, drains
accepted requests, and awaits dispatcher and connection closure. Dropping the
service owner signals shutdown, but explicit shutdown is needed to await it and
receive close errors.

The database uses schema version 7 and a five-second busy timeout. Up-to-date
schema checks avoid a writer transaction; migrations recheck the version inside
their transaction. Standalone persistence functions also use SQLx repositories
and explicitly close their short-lived connections.

### Instance profiles

The `instances` table is keyed by `(instances_directory, instance_folder)`.
Profile settings have dedicated columns; JVM argument lists and optional
installation selection/state use JSON columns. Reads and writes share validation.
An asynchronous mutation lock prevents settings edits and background preparation
from overwriting each other's changes.

Deleting a profile and its modpack index is one database transaction after
filesystem cleanup succeeds. Missing registered directories are omitted from the
list. Duplication copies game files and creates a separate database profile,
without copying obsolete root profile files or the installation workspace.

New instances never write `instance.json`. Existing profile files are neither
imported nor removed, and folders without a SQLite profile are not automatically
registered. The CLI can still launch an unregistered game directory using global
settings. Legacy game-directory behavior is described in
[Compatibility](COMPATIBILITY.md#launchwrapper-game-directory).

## Verification

Offline relocation tests use temporary directories:

```sh
cargo test -q -p licht-core --lib store::layout::relocation
cargo test -q -p licht-core --lib store::cache::override_tests
pnpm vitest run src/features/settings/sections/downloads-section.test.tsx
```
