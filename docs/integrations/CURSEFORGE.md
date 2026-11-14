# CurseForge API

Licht can list, show and import CurseForge modpacks on the Discover page when an API key is available.

## Key

Set `LICHT_CURSEFORGE_API_KEY` in the environment or a local `.env`. The value is never logged.

Without the key, [Modrinth](MODRINTH.md) search and `.mrpack` import still work; CurseForge results are skipped.

## Terms

Using the CurseForge Core API is subject to Overwolf / CurseForge terms and the API program rules in force when you request a key. Do not redistribute Minecraft assets. Review their current developer documentation before shipping a public build that embeds or distributes a key.

This project does **not** embed a CurseForge key in the repository. Forks and official builds that want CurseForge must supply their own key through the environment (or a future approved embed, if the program allows it).

## Scope in Licht

- Search Minecraft modpacks with `index`/`pageSize` (10 per page, merged with Modrinth's page of the same number). CurseForge rejects searches past `index + pageSize > 10000`, so its share of the page count is capped there and later pages only show Modrinth results.
- Optional filters map to `gameVersion`, `modLoaderType` (Forge 1, Fabric 4, Quilt 5, NeoForge 6) and `categoryIds` (a `[id,id]` list; CurseForge returns only packs that have every listed category). Each search accepts one loader, so several chosen loaders become one search each and the pages are merged; a pack that matches more than one loader can be counted twice in the total. Sorting maps to `sortField` 6 (total downloads), 3 (last updated) or 2 (popularity, used for relevance, since there is no text relevance sort).
- Licht's shared categories map to CurseForge modpack category slugs (Adventure → `adventure-and-rpg`, Challenging → `hardcore`, Combat → `combat-pvp`, Kitchen sink → `extra-large`, Lightweight → `small-light`, Technology → `tech`; Magic, Multiplayer and Quests keep their names). Their ids are read once per run from `GET /v1/categories?gameId=432&classId=4471`. Optimization has no CurseForge match, so a filter that includes it searches Modrinth only, as does a slug the API no longer lists (logged as a warning).
- Without a key, CurseForge is skipped silently unless the search asks for CurseForge only, which reports the missing key.
- Load a pack's details with `GET /v1/mods/{id}` and `GET /v1/mods/{id}/description`; the mod must be in the modpack class. The HTML description is sanitized with `ammonia` (no scripts, frames, styles or event handlers; HTTPS links and images only), and only HTTPS screenshots and website links are kept. Details also carry the shared categories (by slug), loaders and game versions from `latestFilesIndexes`, the creation and modification dates, and the wiki, issues and source links. CurseForge has no follower count or license in this response.
- List every file of a pack with `GET /v1/mods/{id}/files`, 50 per request (the API maximum) until `totalCount`, capped at 1,000 files. The first page's `totalCount` says which pages follow, and those are fetched up to four at a time. The id must be numeric. Each file keeps its game versions, loaders (CurseForge mixes their names into `gameVersions`), release channel (`releaseType` 1/2/3), date and downloads, newest first.
- Download a pack archive from the CurseForge CDN over HTTPS. Allowed hosts are `edge.forgecdn.net`, `mediafilez.forgecdn.net`, and `media.forgecdn.net`, checked again on every redirect. The API key is sent as the `x-api-key` header and is not written into the instance or the logs.
- Read `manifest.json`: Minecraft version, overlays from `overrides/` (and `override/`), and the `files` list.
- Resolve each file with `POST /v1/mods/files` and `POST /v1/mods`. SHA1 is checked when the API sends one; MD5 is checked when that is the hash available. The resolved list is stored in the `curseforge_files` table of `licht.db` (see [Storage](../internals/STORAGE.md#modpack-indexes)) and downloaded on install, in the same pass as `.mrpack` files. An instance still holding a `licht-curseforge.json` from v0.2.x has it imported into SQLite and removed on its next install or repair.
- A null or empty `downloadUrl` means the author disabled third-party distribution. Those files are not downloaded and the URL is not guessed. Import stops before the instance is created and the error names each blocked mod.
- The importer uses the Minecraft version and supported primary loader declared in the manifest to prepare the instance's version profile. Loader installation is documented in [Loaders](../internals/LOADERS.md).
