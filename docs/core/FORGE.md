# Forge and NeoForge installation

The New Instance wizard and `.mrpack` loader dependencies use the official
Forge/NeoForge installer metadata. Selecting a loader writes its real version
profile, not a vanilla placeholder. Client processors run during Install/Repair,
before the profile is considered ready to launch.

## Pipeline

1. Download the installer JAR from the family's official Maven and verify its
   published `.sha1` sidecar. A verified cached installer can be reused.
2. Read `install_profile.json` and the referenced version JSON. Older installers
   can carry `versionInfo` inline. Preserve the loader's main class, JVM/game
   arguments, and libraries.
3. Install the inherited vanilla version from Mojang and prepare the required
   Java runtime. Loader installation uses a configured global Java for that
   major when available, otherwise the cached/downloaded Temurin runtime.
4. Extract embedded Maven libraries and download installer/runtime dependencies,
   verifying SHA1. Paths are validated before writing into the cache.
5. Resolve client-side data, Maven references, literals, and installer resources.
   Skip server-only data and processors.
6. Execute client processors sequentially with their JAR manifest's `Main-Class`
   and declared classpath. Arguments are passed individually, without a shell.
   Missing dependencies, unresolved tokens, nonzero exits, missing outputs, and
   hash mismatches fail installation.
7. Download Mojang client mappings through Licht's verified downloader instead
   of letting the mapping processor fetch them without the launcher checking
   the published hash.
8. Verify declared processor output hashes and final hashes stored in installer
   data. Install the resolved game files, then write the completion receipt.

Minecraft files still come from Mojang. Loader installers and their libraries
come from the sources declared by official installer metadata.

## Cache and readiness

Each loader version directory contains `installer.jar`, `installer.sha1`,
extracted `processor-data`, its version JSON, and `licht-loader-install.json`.
Libraries and processor outputs use the shared `libraries/` Maven layout.

The receipt records SHA1 of the profile, client JAR, downloaded libraries, and
generated outputs. Outputs without a vendor-declared hash are checked for
successful production and then recorded with their computed SHA1 for later
integrity checks. Readiness verifies the receipt's hashes. A vanilla parent JAR
alone no longer makes a provisional Forge/NeoForge profile ready.

A valid receipt skips the processor pipeline. Declared valid processor outputs
can also be reused during repair. Missing or modified receipt files are repaired
by running Install/Repair again. Different loader builds can share intermediate
Minecraft libraries, so loader installation is serialized within the process.

## Existing instances and launch

IDs remain `forge-<build>` and `neoforge-<build>`. Install/Repair replaces old
`lichtLoader` vanilla placeholders with official profiles without changing the
instance's folder or settings. Forge builds from modpacks can omit the Minecraft
prefix; it is added to the Maven URL without changing the stored instance ID.

Version inheritance prefers a child's version of the same Maven artifact and
classifier. Launch derives `${library_directory}` and `${classpath_separator}`
from the cache and operating system. Windows installation paths are made absolute
without adding `canonicalize`'s `\\?\` prefix, which Java's classpath reader rejects.

The existing Tauri commands and progress events are retained. Byte/file progress
uses the normal download pipeline; processors use the existing Preparing state.
Installer orchestration is in `licht-core`, not in the Tauri adapter.

## Verification

Automated tests use installer JSON fixtures and synthetic JARs without accessing
the network. They cover metadata formats, client-side selection, coordinates,
tokens, paths, checksums, classpath separators, processor failure, inheritance,
and receipt invalidation.

Live Windows installation and offline launch checks used an isolated temporary
cache and the same core pipeline invoked by Tauri:

| Loader | Minecraft | Build | Result |
| --- | --- | --- | --- |
| Forge | 1.12.2 | 14.23.5.2860 | Forge Mod Loader, resources, and audio initialized |
| Forge | 1.20.1 | 47.3.0 | ModLauncher, MinecraftForge, resources, and audio initialized |
| NeoForge | 1.21.1 | 21.1.80 | ModLauncher, NeoForge, resources, and audio initialized |

Each client remained running for 45 seconds before the validation process was
stopped. A repeat verified install reused its cached results. A deliberately
corrupted NeoForge generated client library was rejected and repaired. An old
Forge provisional profile was rejected and upgraded to its real main class.
Linux graphical launch remains unverified; CI checks compilation and tests on
Windows and Ubuntu. These checks do not claim coverage of every historical
Forge installer or Minecraft/loader combination.
