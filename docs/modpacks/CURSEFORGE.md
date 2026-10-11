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
- Download a pack archive and create an instance (game version from `manifest.json`, overlays from `overrides/`).
- Full CurseForge dependency resolution and Forge installer processors may land in later work.
