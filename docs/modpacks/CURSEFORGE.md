# CurseForge API

Licht can list and import CurseForge modpacks in the New Instance wizard when an API key is available.

## Key

Set `LICHT_CURSEFORGE_API_KEY` in a local `.env` (see `.env.example`). The value is never logged.

Without the key, Modrinth search and `.mrpack` import still work; CurseForge results are skipped.

## Terms

Using the CurseForge Core API is subject to Overwolf / CurseForge terms and the API program rules in force when you request a key. Do not redistribute Minecraft assets. Review their current developer documentation before shipping a public build that embeds or distributes a key.

This project does **not** embed a CurseForge key in the repository. Forks and official builds that want CurseForge must supply their own key through the environment (or a future approved embed, if the program allows it).

## Scope in Licht

- Search Minecraft modpacks and list file versions.
- Download a pack archive from the CurseForge CDN over HTTPS. Allowed hosts are `edge.forgecdn.net`, `mediafilez.forgecdn.net`, and `media.forgecdn.net`, checked again on every redirect. The API key is sent as the `x-api-key` header and is not written into the instance or the logs.
- Read `manifest.json`: Minecraft version, overlays from `overrides/` (and `override/`), and the `files` list.
- Resolve each file with `POST /v1/mods/files` and `POST /v1/mods`. SHA1 is checked when the API sends one; MD5 is checked when that is the hash available. The resolved list is stored in `licht-curseforge.json` and downloaded on install, in the same pass as `.mrpack` files.
- A null or empty `downloadUrl` means the author disabled third-party distribution. Those files are not downloaded and the URL is not guessed. Import stops before the instance is created and the error names each blocked mod.
- Mod loader install and Forge installer processors are still later work. The instance keeps the Minecraft version from the manifest.
