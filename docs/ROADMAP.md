# ROADMAP: Licht Launcher

Unofficial Minecraft: Java Edition launcher for **Windows and Linux**, built with Tauri v2 (Rust + TypeScript).

**Frontend stack:** React + TypeScript + Vite, Tailwind CSS, shadcn/ui (Radix UI), Lucide (icons), TanStack Router (file-based routes, hash history), TanStack Query (`invoke` calls), Vitest + Testing Library (tests). Installed, not used yet: TanStack Virtual. Planned, not installed: Zustand (global state).

> **How to use:** always ask for **one item at a time** ("do item 3.2"). Flow for each item: **plan → approval → implementation → PR → review**. Follow `AGENTS.md`.

Legend: `[ ]` pending · `[x]` done · 🛑 = decision that requires explicit approval before coding · 📚 = Rust concepts that show up

---

## Identity and conventions

| Item                  | Value                                                                           |
| --------------------- | ------------------------------------------------------------------------------- |
| Display name          | **Licht Launcher** (_Licht_ = "light" in German; adopted pronunciation: "líkt") |
| Repository / packages | `licht-launcher`                                                                |
| Binary / command      | `licht`                                                                         |
| Core crate            | `licht-core`                                                                    |
| Symbol (logo)         | a lily made of light                                                            |
| Required notice       | "Unofficial project, not affiliated with Mojang Studios or Microsoft"           |

> 🛑 Before item 0.1: confirm the domain (e.g. `licht.app`, `getlicht.com`, or a `.com.br` at registro.br) and the app _bundle identifier_ (e.g. `app.licht.launcher`).

### Version codenames

Each `0.x` release gets the name of a flower, in German, in this order:

| Version | Codename      | Meaning    |
| ------- | ------------- | ---------- |
| 0.1     | **Lilie**     | lily       |
| 0.2     | **Iris**      | iris       |
| 0.3     | **Rose**      | rose       |
| 0.4     | **Immergrün** | periwinkle |
| 0.5     | **Anemone**   | anemone    |
| 0.6     | **Narzisse**  | daffodil   |
| 0.7     | **Edelweiß**  | edelweiss  |

The codename appears in the GitHub release title and in the installer name (e.g. `Licht-Launcher-0.1.0-Lilie`). From 0.8 on, the list continues (🛑 define together).

---

## Phase 0. Foundation — closed

**Goal:** repository ready, with automated quality from the first commit.

- [x] **0.1** Create the workspace: `crates/licht-core` (lib), `src-tauri` (app), `src/` (TS + Vite frontend).
  - 🛑 Package manager (pnpm) and frontend folder structure (`src/routes`, `src/components/ui`, `src/features/*`, `src/lib`).
  - 📚 Cargo workspace, lib vs bin crates.
- [x] **0.2** Tooling: `rustfmt`, `clippy -D warnings`, Biome (lint and formatting), strict TypeScript, Vitest + Testing Library.
- [x] **0.3** CI (GitHub Actions) with a **windows-latest + ubuntu-latest** matrix: fmt, clippy, Rust and frontend tests, Tauri build.
- [x] **0.4** Logging structure (`tracing`) and base error type (`thiserror`) in the core.
  - 📚 `Result`, the `?` operator, `thiserror` vs `anyhow`.
- [x] **0.5** Short `docs/ARCHITECTURE.md` describing the layers (core / tauri / frontend).
- [x] **0.6** Project license and "unofficial" notice in the README.
  - 🛑 License (MIT, Apache-2.0, GPL...).
- [x] **0.7** **Base design system**: Tailwind + shadcn/ui configured, **design tokens** (colors, radii, spacing, typography) as CSS variables in a single file, light/dark theme, Lucide icons, and base components (Button, Input, Dialog, Tabs, Progress, Toast, Select, Tooltip, ScrollArea).
  - 🛑 **Tailwind version (v3 or v4):** test on an older WebKitGTK (e.g. Ubuntu 22.04) before deciding. If there is a problem, use v3.
  - 🛑 Visual direction: palette, typography, light/dark, level of minimalism (starting point: lily of light, soft tones).
  - Avoid heavy effects (`backdrop-filter`, blurs, and large shadows) because of WebKitGTK on Linux.

**Closed.** The main window opens empty, CI covers Windows and Ubuntu, and the design system lives in `src/components/ui` with tokens in `src/styles/tokens.css`.

**Context beyond the items:** TanStack Router was installed in this phase (`@tanstack/react-router` and the Vite plugin). Routes are files in `src/routes`; screens stay in `src/features`. The tree generated in `src/routeTree.gen.ts` is committed, so `tsc` does not depend on a Vite build. History is hash-based (`/#/`) because the Tauri window does not rewrite paths to `index.html`. TanStack Query is installed and used for `invoke` calls (phase 5+). Zustand remains decided and not installed. TanStack Virtual is installed but not wired yet. Interface text is in English.

---

## Phase 1. Mojang data (parsing)

**Goal:** read and understand the manifests, with no network in tests.

- [x] **1.1** `serde` types for `version_manifest_v2` + HTTP client (`reqwest`) to fetch it.
  - Fixture: save a real manifest in `tests/fixtures`.
  - 📚 `serde`, `struct`/`enum`, `async/await` with `tokio`.
- [x] **1.2** Types for a **version JSON** (libraries, arguments, assetIndex, mainClass, downloads, javaVersion). Test with fixtures of old versions (e.g. 1.8, 1.12) and new ones (1.20+, 1.21).
  - 🛑 Strategy for versions with the old format (`minecraftArguments`) vs the new one (`arguments`).
- [x] **1.3** **`rules`** evaluator (os/arch/features) with tests covering Windows, Linux, and different architectures.
  - 📚 `match`, `Option`, simple traits.
- [x] **1.4** Types and reading of the **asset index**.

**Done when:** a test loads the JSON of any fixture version and lists the libraries that apply to the current OS.

---

## Phase 2. Downloads

**Goal:** download everything in a way that is fast, verified, and resumable.

- [x] **2.1** Single-file downloader: stream to disk, **SHA1 verification**, atomic write (`.part` → rename), retry with backoff.
  - 📚 Buffer ownership, `Path`/`PathBuf`, `AsyncRead`.
- [x] **2.2** Parallel queue with a concurrency limit (e.g. 8-16) and progress aggregation.
  - 🛑 Default concurrency limit and retry policy.
  - 📚 `tokio::spawn`, `Semaphore`, channels (`mpsc`).
- [x] **2.3** Directory structure and **shared cache** (libraries, assets by hash) outside instances.
  - 🛑 On-disk layout (compatible or not with the official `.minecraft`).
  - Use `directories` for per-OS paths.
- [x] **2.4** Version installer: given a version ID, downloads client.jar + libraries + assets and skips what is already valid.

**Done when:** installing the same version twice downloads everything the first time and nothing the second time.

**In review:** the resumable engine ([#105](https://github.com/licht-launcher/licht-launcher/pull/105)),
persistent jobs ([#106](https://github.com/licht-launcher/licht-launcher/pull/106)), and
managed installations with UI controls ([#107](https://github.com/licht-launcher/licht-launcher/pull/107))
add pause, cancellation, and manual recovery after restart. Manual validation on
Windows and Linux is still required. A dedicated download-history page remains planned.

---

## Phase 3. Java

**Goal:** have the right Java for each game version, without the user installing anything.

- [x] **3.1** Discover the required runtime (`javaVersion` from the version JSON).
- [x] **3.2** Download and install Adoptium/Temurin for the required major (Mojang's own `java-runtime` packages are not used).
  - Decision: Eclipse Temurin by default, JRE when Adoptium publishes one, otherwise the JDK. The major version comes from `javaVersion.majorVersion`, or from the version id when that field is absent. `--java` uses the given executable. A Java already installed on the machine is not selected on its own.
- [x] **3.3** Detect already installed Javas (optional) and validate the version.

**Done when:** asking for "1.21" and "1.8" results in two distinct, correct, usable runtimes.

---

## Phase 4. Game assembly and execution

**Goal:** open Minecraft.

- [x] **4.1** Extraction of **natives** per platform into `natives/<version id>` in the data directory. A later launch extracts over the same folder.
  - 📚 `zip`, I/O error handling.
- [x] **4.2** **Classpath** assembly (`;` separator on Windows, `:` on Linux) and the full command: JVM arguments + game arguments, with variable substitution (`${auth_player_name}`, `${game_directory}`, `${assets_root}`...).
- [x] **4.3** Process execution (`std::process`/`tokio::process`), stdout/stderr capture, end/crash detection.
- [x] **4.4** CLI `licht` in `licht-core` launches a cached version with an offline account (chosen name). Offline is a product launch mode; official Microsoft login stays in phase 7. `licht launch` does not download. It only opens a version already in the cache.
- [x] **4.5** Natives for current versions. 26.3 sets `-Djava.library.path` to `${natives_directory}/java`, and LWJGL 3.4 stores `lwjgl.dll` under `windows/x64/...` inside the jar. Extraction flattens that DLL into `natives/<id>`, and the launch rewrites the flag to that folder. Older versions already store the library at the jar root.
- [x] **4.6** `licht install --version <id>` installs one version. Mojang is the only source: download the version JSON, write `versions/{id}/{id}.json`, download the client, libraries and assets, and install the Temurin runtime for that version. `--java` skips Temurin and uses the given executable. Files go to Licht's data directory, not the official `.minecraft` folder. The versions screen (5.3) calls the same function.
- [x] **4.7** `licht versions` lists id and type from Mojang's manifest and does not install. `licht launch` can omit `--java` when that version's runtime is already in the cache.

### 🏁 Milestone 1: the game opens

Install and open **vanilla 1.21.x** and **an old version (1.8.9)** on Windows and Linux, from the CLI.

Reached on Windows, from `licht`, with an offline name: 26.3, 1.20.1, 1.12.2, 1.8.9, 1.7.2, and 1.5.2 opened. 1.7.2 entered a world and exited 0. Linux has not been launched in this pass.

Also in place, beyond the original checklist: one `instances/<id>` folder per version (not the phase 6 profile model), install progress in megabytes, natives limited to this machine, and legacy sounds copied to the names old clients open. Details are in [COMPATIBILITY.md](COMPATIBILITY.md) and [LAUNCHWRAPPER.md](core/LAUNCHWRAPPER.md).

Still open from this milestone: `${auth_session}` stays literal on 1.5.2. Linux launch was not re-checked in that pass. Account login is phase 7.

---

## Phase 5. Basic interface — closed

**Goal:** use all of this through a UI.

- [x] **5.1** Tauri commands (`list_versions`, `install_version`, `launch`), thin, only calling the core. Types shared with the frontend via `ts-rs`.
  - 📚 `#[tauri::command]`, `State`, `Arc`/`Mutex`.
- [x] **5.2** Progress events (download, installation, game status) emitted from Rust to the frontend, with **throttling** (~100 ms) so the UI is not saturated.
- [x] **5.3** Play surface: install, play/stop, progress. Evolved into the **instances hub** (sidebar + play panel) instead of a standalone versions list. Release/snapshot filter lives in the create-instance dialog. TanStack Virtual is installed but not wired (lists are still short).
- [x] **5.4** Real-time game log console (per instance, with level filter).
- [x] **5.5** Friendly error handling in the UI (network down, disk full, invalid SHA1) via toasts.
- [x] **5.6** Visual identity: lily logo and application icons generated for the supported platforms and sizes. The palette and tokens have existed since item 0.7.

**Done when:** it is possible to install and play a version from the interface alone. **Reached** via the instances hub (offline username).

**Context beyond the items:** stop/orphan recovery for running games, install progress with version id (parallel installs), and download/write concurrency split in settings.

---

## Phase 6. Instances — closed

**Goal:** isolated profiles.

- [x] **6.1** Instance model (name, version, own folder, min/max RAM, extra JVM arguments, resolution), persisted in versioned `instance.json`.
- [x] **6.2** Instance CRUD (create, duplicate, rename, delete) in the core + UI.
- [x] **6.3** Global settings (data folder, Java, download concurrency, write concurrency, theme). Per-instance launch overrides are also in place.
- [x] **6.4** Open the instance folder in the OS file manager.

**Closed.** Instances are the primary product surface. Settings open in a dialog with sectioned auto-save. Custom title bar, Files tab, and last-played sort landed on `main`.

---

## Phase 7. Microsoft login

**Goal:** legitimate accounts.

Decisions (see [docs/auth/MICROSOFT.md](auth/MICROSOFT.md)): authorization code + PKCE (system browser / localhost), scopes `XboxLive.signin offline_access`, client ID embedded (+ optional `LICHT_MSA_CLIENT_ID`), tokens in the OS keyring.

- [x] **7.1** Register an app in Azure and request approval to access the Minecraft services API. **Worth starting this request early**, in parallel with the previous phases, because it can take a while.
- [x] **7.2** OAuth authorization code + PKCE (browser) → Xbox Live → XSTS → Minecraft Services → profile (name, UUID, skin).
- [x] **7.3** Secure token storage with the **OS keyring** (Credential Manager / Secret Service). Never in plain text, never in logs.
- [x] **7.4** Automatic renewal (refresh token), multiple accounts, logout.
- [x] **7.5** Check whether the account owns the game, and clear error messages.

### 🏁 Milestone 2: usable launcher with a real account

---

## Phase 8. Mod loaders

- [x] **8.1** **Fabric** (metadata API): install the loader on an instance.
- [x] **8.2** **Quilt** (same approach).
- [x] **8.3** **NeoForge/Forge** (official installer profiles, verified libraries, client processors, completion receipts, and repair of provisional profiles). See [FORGE.md](core/FORGE.md) for the pipeline and validated combinations.
  - 🛑 Included in the New Instance wizard scope.
- [x] **8.4** Loader and loader-version selection when creating an instance (`/instances/new`).

---

## Phase 9. Mods and modpacks

- [x] **9.1** **Modrinth** API client: search modpacks and list versions (mod install UI still open).
- [ ] **9.2** Install/update/remove mods on an instance, with hash verification and dependency resolution.
  - Content listing, enable/disable, and removal are available. Mod installation,
    updates, and dependency resolution remain pending.
- [x] **9.3** Import `.mrpack` modpacks.
- [x] **9.4** CurseForge modpack search/import via `LICHT_CURSEFORGE_API_KEY` (see `docs/modpacks/CURSEFORGE.md`).
  - 🛑 Terms documented; no key in the repo.

---

## Phase 10. Polish and distribution

- [ ] **10.1** Installers: Windows (NSIS/MSI) and Linux (AppImage + deb; Flatpak as a goal).
  - [x] Windows NSIS, Windows portable, Linux AppImage, Linux deb, and portable artifacts are generated by CI.
  - [ ] MSI and Flatpak remain additional distribution formats.
- [x] **10.2** Auto-update (Tauri updater) with signing.
  - GitHub Releases is the update host.
  - The public key is committed; the private key is kept in GitHub Actions
    secrets and is never committed.
  - Windows/NSIS and Linux/AppImage updater artifacts are generated and signed by
    the release workflow. Settings and the title bar provide update actions.
    Publisher signing is separate; see [CODE_SIGNING.md](CODE_SIGNING.md).
- [x] **10.3** Automated release in CI (tag → build → GitHub Release), with the version codename in the title.
- [x] **10.4** No product analytics or automatic crash-report uploads. See [PRIVACY.md](../PRIVACY.md).
- [x] **10.5** README, unofficial-project notice, MIT license, contribution guide, Terms of Use, Privacy Policy, and code-signing documentation.
- [ ] **10.6** Performance review: startup time, idle RAM, download throughput.

### 🏁 Milestone 3: public v1.0

---

## Future ideas (out of the initial scope)

- World backup/restore
- Import instances from other launchers (Prism, MultiMC)
- Skins and cape management
- macOS support
- Plugins/themes
- Favorite servers with status ping

---

## References

- Version manifest: `https://piston-meta.mojang.com/mc/game/version_manifest_v2.json`
- minecraft.wiki (JSON formats and authentication)
- Reference code: Prism Launcher, HMCL, portablemc, minecraft-launcher-lib
- Tauri v2: https://v2.tauri.app
- The Rust Book (chapters 4, 6, 9, and 16 are the most useful for this project)

## Standing rules

1. One item = one branch = one small PR.
2. No code outside the item's scope.
3. All core logic has tests; no test depends on the network.
4. Game data comes only from Mojang's servers; never redistribute Minecraft files.
5. Always ask on decisions marked with 🛑.
