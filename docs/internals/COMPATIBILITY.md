# Compatibility

This document records version differences that affect installation and launch.
Game files come from Mojang and are not redistributed. Java selection lives in
[Runtime](RUNTIME.md), directory layout in [Storage](STORAGE.md), and mod-loader
installation in [Loaders](LOADERS.md).

Sources: minecraft.wiki ([client.json](https://minecraft.wiki/w/Client.json),
[Update Java](https://minecraft.wiki/w/Tutorial:Update_Java)) and historical Windows
checks of 26.3, 1.21.11, 1.20.1, 1.12.2, 1.8.9, 1.7.2, and 1.5.2. These checks do
not establish Linux graphical compatibility or coverage of every version.

## Manifest and version ids

The manifest contains `release`, `snapshot`, `old_beta`, and `old_alpha` entries.
Version ids are opaque strings, including `1.5.2` and `26.3`.
`latest.release` and `latest.snapshot` refer to entries in the same list.

`licht versions` lists every entry as `<id> <type>` without installing anything;
`--cache` does not affect the list. `licht install --version <id>` finds the entry,
downloads its JSON, verifies the manifest's SHA1, and writes
`versions/<id>/<id>.json`. Unknown ids and hash mismatches fail installation.
`licht launch` reads the cached JSON without downloading the list; a missing
version fails launch. Empty ids, `.`, `..`, and ids containing a slash are rejected.

Install/launch do not branch on `complianceLevel`. The version id's shape affects
Java selection only when `javaVersion` is absent; see [Runtime](RUNTIME.md#which-major).

## Arguments

Through 1.12.2, game arguments are the `minecraftArguments` string, split on ASCII
whitespace. There are no manifest JVM arguments: the core inserts
`-Djava.library.path=${natives_directory}`, `-cp`, the classpath, then `mainClass`
and the game arguments.

Since 1.13 (`17w43a`), `arguments.game` and `arguments.jvm` contain strings or
objects with `rules` and `value`. The core evaluates rule-gated entries.
Versions through 1.5.2, except snapshot `13w16b`, use
`net.minecraft.launchwrapper.Launch`; later versions use
`net.minecraft.client.main.Main`.

## Offline account

The offline id is a version 3 UUID of the UTF-8 string `OfflinePlayer:<name>`,
written without hyphens. The access token is `0`, `${user_type}` is `legacy`, and
`${user_properties}` is `{}`. Modern `${auth_xuid}` and `${clientid}` are empty.

The 1.5.2 arguments are
`${auth_player_name} ${auth_session} --gameDir ${game_directory} --assetsDir ${game_assets}`.
The expected `${auth_session}` value is `0:<uuid>`, but the core currently leaves
that placeholder literal. From 1.7, arguments use `${auth_access_token}` and
`${auth_uuid}` instead. Microsoft accounts are covered in
[Microsoft authentication](../integrations/MICROSOFT.md).

## LaunchWrapper game directory

The core recognizes LaunchWrapper by its main class or a library name beginning
with `net.minecraft:launchwrapper:`. The injector applies `--gameDir` after
`Minecraft.main` returns, so the client thread can read options from the default
directory first. A 1.5.2 client can then read `lang:en_us` from an existing official
installation and fail because its jar contains `lang/en_US.lang` instead.

Licht uses `<instance>/.minecraft` for these versions. The process working
directory and `${game_directory}` point there; `APPDATA` and `-Duser.home` point
to its parent. Both Windows and Linux default-directory resolution then reach
the same folder as `--gameDir`. Other versions use the instance directory itself.
`--game-dir` replaces the CLI's instance root. The official installation is not
modified, and Licht does not ship a Java launcher or create a junction.

Reference approaches considered: HMCL redirects `APPDATA`/`user.home`; Prism and
MultiMC use a Java legacy launcher to set the directory before game startup.
Passing only `--gameDir`, as in minecraft-launcher-lib and portablemc, does not
close this race for a separate directory. These are historical design references,
not assertions about every current upstream release.

## Assets

Assets download from `resources.download.minecraft.net` into
`assets/objects/<first two hash characters>/<hash>`. Old clients require logical
names such as `sounds/random/click.ogg`; leaving only hashed files makes them
start without sounds and attempt the obsolete MinecraftResources endpoint.

| Asset index | Versions | Materialized location |
| --- | --- | --- |
| `pre-1.6`, `map_to_resources` | Classic, alpha, beta, releases through 1.5.2, snapshots through `13w23b` | `<game directory>/resources` |
| `legacy`, `virtual` | `13w24a` through 1.7.2 | `<data>/assets/virtual/legacy` |
| Version-specific | From 1.7.3 | Hashed objects remain in place |

For old indexes, launch copies objects to their logical paths and sets
`${game_assets}` to the materialized location. LaunchWrapper's 1.5.2 resources
therefore live in `<instance>/.minecraft/resources`. Modern clients receive
`${assets_root}` and `${assets_index_name}` without copies. If both legacy flags
are set, both copies are made and `${game_assets}` uses the virtual folder.

A destination with the expected size is reused without rehashing; the download
verified SHA1, but a later same-size replacement would be retained. Missing objects
fail launch. Logical paths reject empty segments, `.`, `..`, backslashes, colons,
and absolute segments.

Historical Windows validation: 1.5.2 reached the menu with sound; 1.7.2 reported
`Sound engine started`, entered a world, and exited 0.

## Natives

Install downloads the native jar for the current OS/architecture. Launch prepares
`.dll`, `.so`, and `.dylib` files in a cached generation under
`natives/<version id>/<os>-<arch>/<generation>` and passes that directory as
`-Djava.library.path`. A fingerprint of the platform, input hashes, and extraction
rules selects the cached generation. Unchanged managed outputs are reused;
missing or changed outputs cause a new extraction. Source jars are SHA1-verified
before extraction, and generation metadata is persisted in SQLite.

- **Legacy libraries:** the `natives` map supplies classifiers such as
  `natives-windows-${arch}`, with `64` or `32`. The classifier jar is extracted;
  the plain library jar remains on the classpath.
- **Current libraries:** the Maven classifier, such as `natives-windows`, names
  the native jar. That jar is extracted and omitted from the classpath. The plain
  Windows classifier is 64-bit; `natives-windows-x86` is 32-bit. Selection supports
  `x86` and `x86_64`, not arm64.

Extraction drops `META-INF`, non-native entries, and `extract.exclude` prefixes.
Nested paths are flattened; entries that escape the destination fail launch.
26.x/LWJGL 3.4 stores DLLs under `windows/x64/...` and requests
`${natives_directory}/java`. Since extraction flattens them, launch rewrites that
argument to the version's native folder. 1.8.9 and 1.21 use root-level binaries
and keep the ordinary `${natives_directory}` argument.

## Log encoding

One codec covers both stdout and stderr for a game process. The major comes from
the chosen runtime or `java -version` when `--java` is supplied. Java 18 and newer
use UTF-8. For older Java on Windows, `GetACP()` selects Windows-1252 for page 1252,
UTF-8 for page 65001, and lossy UTF-8 for other pages. Linux uses lossy UTF-8.

Invalid bytes become U+FFFD rather than stopping log reading. Undefined
Windows-1252 bytes in `0x80`–`0x9F` also become U+FFFD. This fixed a strict UTF-8
read failure on `INFORMAÇÕES` in the Windows 1.5.2 log. Licht does not pass
`-Dfile.encoding`: decoding changes how the launcher reads pipes, not the game.
Java 8 identifying Windows 11 as Windows 8.1 is not itself a launch failure.

## Known gaps

- `${auth_session}` remains literal in legacy arguments such as 1.5.2's.
- Java 17 and older use one codec for the whole process. Mixed UTF-8 output can
  be misread as Windows-1252; unsupported Windows code pages use lossy UTF-8.
- The historical launch checks above were performed on Windows. Linux graphical
  launch was not re-checked in that pass.
