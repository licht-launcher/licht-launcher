# Modrinth

Licht searches Modrinth modpacks for the Discover page, loads each pack's details,
lists its versions, and imports `.mrpack` archives. No API key or Modrinth account is required for
these public API requests. [CurseForge](CURSEFORGE.md) is a separate integration.

## API and downloads

- The client uses `https://api.modrinth.com/v2` and identifies Licht through a
  `User-Agent` containing the project name and repository URL.
- Search filters to `project_type:modpack` and pages with `offset`/`limit` (10
  results per page, merged with CurseForge's page of the same number; `total_hits`
  sets the page count). Optional filters become ANDed facets: `versions:<game
  version>`, `categories:<loader>` (loaders are categories on Modrinth) and
  one `categories:<category>` facet per chosen category (so a pack must have
  all of them) from Licht's shared category list, which mirrors
  Modrinth's modpack categories. The sort maps to `index=downloads|updated|relevance`.
  Version lists come from `GET /v2/project/{id}/version` (the id percent-encoded
  as one path segment) and expose game versions, loaders, the release channel
  (`version_type`), the publish date and downloads, newest first.
- Pack details come from `GET /v2/project/{id}`; the id is percent-encoded as a
  single path segment and the project must be a `modpack`. The author name comes
  from a search filtered by `project_id`, because organization-owned projects
  have no owner in their member list. Details also carry followers, the license,
  publish and update dates, supported game versions (newest first), loaders,
  shared categories (including `additional_categories`) and the source, issues,
  wiki and Discord links. Only HTTPS gallery images and links are kept.
- The Markdown `body` is rendered with `pulldown-cmark` and sanitized with
  `ammonia` before it reaches the UI: scripts, frames, styles and event handlers
  are removed, and links and images must be absolute HTTPS URLs.
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
