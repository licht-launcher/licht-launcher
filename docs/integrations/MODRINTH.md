# Modrinth

Licht searches Modrinth modpacks in the New Instance wizard, lists their versions,
and imports `.mrpack` archives. No API key or Modrinth account is required for
these public API requests. [CurseForge](CURSEFORGE.md) is a separate integration.

## API and downloads

- The client uses `https://api.modrinth.com/v2` and identifies Licht through a
  `User-Agent` containing the project name and repository URL.
- Search filters to `project_type:modpack`, sorts by downloads, and requests up to
  20 results. Version lists expose game versions and loader names.
- Before downloading a selected archive, the client verifies that its version
  belongs to the project. It chooses the primary file, or the first file when no
  primary is marked.
- Archive downloads require HTTPS on `cdn.modrinth.com`, including redirects.
  They require a SHA1 or SHA512 checksum; SHA512 is additionally checked whenever
  supplied by the API.

## Import and installation

The importer reads `modrinth.index.json`, sets the Minecraft version, and extracts
`overrides/` followed by `client-overrides/`. Paths are validated to prevent
escaping the instance, targeting launcher-owned files, or using Windows device
names. The parsed pack index is persisted through the SQLite service rather than
writing a new `licht-modpack.json`; see [Storage](../internals/STORAGE.md).

Dependencies select Minecraft and the declared Fabric, Quilt, Forge, or NeoForge
loader. Profile preparation and processor behavior are described in
[Loaders](../internals/LOADERS.md).

Pack file installation skips entries with `env.client: unsupported`. Other entries
require both SHA1 and SHA512, and an allowed HTTPS download URL. Existing files
are checked against both hashes; new downloads are verified before installation
is considered complete.

## Limitations and tests

- Pack file downloads accept only `cdn.modrinth.com`. GitHub, GitLab, and
  CurseForge CDN URLs inside `.mrpack` files are not accepted.
- Modpack dependency selection is distinct from installing/updating individual
  mods and resolving their dependencies. That work remains tracked in
  [Roadmap item 9.2](../ROADMAP.md#phase-9-mods-and-modpacks).
- Automated tests use synthetic archives, metadata, and local files without
  network access. They cover parsing, overrides, paths, hashes, download-host
  restrictions, and loader dependencies.

Implementation: `crates/licht-core/src/modpacks/modrinth.rs` and
`crates/licht-core/src/modpacks/mrpack/`.
