# Licht Launcher (Tauri v2)

## Context
- **Unofficial** Minecraft: Java Edition launcher, cross-platform (**Windows and Linux**), built with **Tauri v2**.
- Frontend: **React + TypeScript + Vite**, **Tailwind CSS**, **shadcn/ui** (Radix UI), **Lucide** (icons), **TanStack Router** (file-based routes in `src/routes`, hash history), **TanStack Query** (installed and used for `invoke` calls), **TanStack Virtual** (installed, but not yet wired to current short lists), and no Zustand dependency. Backend: Rust.
- Display name: **Licht Launcher**. Repository/packages: `licht-launcher`. Binary/CLI: `licht`. Core crate: `licht-core`.
- The project owner is a senior TypeScript/Node developer, but has **LITTLE knowledge of Rust**.
- The full plan is in `docs/ROADMAP.md`. Always work on **one item at a time**.

## How to work with me
- Before any code, present a **short PLAN** (files that will be created/changed, approach, alternatives) and wait for my approval.
- **Ask** when there is a decision: new dependency, architecture change, public API between Rust and the frontend, security/authentication, any ambiguity. Items marked with 🛑 in the roadmap always require my approval. Do not decide on your own.
- One task = one branch = one small, focused PR. Never mix topics.
- **Do not write code beyond what the task asks.** No "for the future" abstractions, no extra features, no refactoring that was not requested.
- Follow the architecture below. If you think it should change, **propose it first**; do not change it.
- File edits inside an already approved plan do not need individual confirmation.
- If something in the task conflicts with this file or the roadmap, stop and tell me.

## Language
- Code, identifier names, comments, and commit messages in **English** (Conventional Commits: `feat:`, `fix:`, `test:`, `chore:`...).
- Interface text in **English**.
- PR descriptions in **English**.
- *(Adjust this section if you prefer another pattern.)*

## Architecture
- Cargo workspace:
  - `crates/licht-core`: all the logic (manifests, downloads, rules, Java, launch, auth). **No Tauri dependency.** Testable on its own.
  - `src-tauri`: thin layer. Only `#[tauri::command]` and events that call `licht-core`. **No business rules here.**
  - `src/`: frontend. No Minecraft logic; only UI and `invoke` calls. Structure: `src/routes` (route files), `src/components/ui` (shadcn/ui), `src/features/*` (screens and UI logic by feature), `src/lib` (utilities and `invoke` wrappers).
- Rust → frontend communication: **events** (e.g. download progress). Frontend → Rust: **typed commands**.
- Types shared between Rust and TS must be **generated** (`ts-rs` or `specta`, to be decided), never duplicated by hand.
- Errors: `thiserror` in the core, `anyhow` only at the edges. **No `unwrap()`** outside tests.
- Logs with `tracing`.
- Use **Tauri v2**. Do not use v1 APIs or examples.

## UI and design system (consistency over creativity)
- Use **only** the components in `src/components/ui` and the **theme tokens** (CSS variables). If a component is missing, propose adding it to the design system instead of creating a one-off style.
- **Forbidden:** inline styles, loose colors/sizes (e.g. `#3b82f6`, `w-[137px]`) outside the tokens, and any new UI library without my approval.
- Before creating a screen, **list the existing components** that will be used and which ones are missing.
- Accessibility: everything operable by keyboard, with visible focus and correct labels (Radix already helps; do not remove that).
- The frontend runs in **WebView2 (Windows)** and **WebKitGTK (Linux)**. Avoid `backdrop-filter`, blurs, large shadows, and complex animations. If you use a modern CSS feature, tell me so I can test it on both systems.
- Calls to Rust via TanStack Query (cache, loading, and error), once Query is installed. Progress events with throttling (~100 ms). Long lists always virtualized.
- Types shared with Rust are generated; never redeclare them by hand.

## Rust (beginner level: prefer simple code)
- Prefer clarity over "advanced idiomatic". Avoid complex lifetimes, custom macros, and `unsafe`. If it is unavoidable, explain it and ask for approval.
- Use `clone()` without guilt when it simplifies things.
- In every PR, include a **"Rust concepts used"** section in the description, explaining in plain language (with a TS/Node comparison when it makes sense) any new concept: ownership, borrowing, `Result`, traits, async/tokio.
- When there is a compiler error, **explain the cause** before fixing it.
- Comment only the non-obvious "why".

## Quality (required before opening a PR)
- Rust: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- Frontend: `pnpm lint`, `pnpm typecheck`, `pnpm test` (Vitest + Testing Library; new components with at least one render/interaction test)
- Automated tests for all core logic. Use **fixtures** (real Mojang JSONs saved in `tests/fixtures`). **No test may depend on the network.**
- If a check fails and you cannot fix it, say so in the PR instead of hiding it.

## PR format
- Short title. Description with: what changed, why, how to test, decisions made, Rust concepts used, points where I want my opinion.

## Domain rules
- Game data **always** comes from Mojang's servers. Never redistribute Minecraft files.
- Verify the **SHA1** of everything that is downloaded.
- No tokens/secrets in the repository or in the logs. Tokens stay in the **OS keyring**.
- Paths and separators must work on Windows and Linux (use `std::path` and the `directories` crate; never concatenate strings).
- The project is **not official**: keep the notice "not affiliated with Mojang Studios or Microsoft" in the README. Do not use "Minecraft" or "Mojang" in the project, package, or binary name, and do not use official logos or art.

## Release conventions
- Each `0.x` release gets a **German flower codename**, according to the table in `docs/ROADMAP.md` (0.1 Lilie, 0.2 Iris, 0.3 Rose...). The codename goes in the release title and in the installer name.

## Useful references
- Manifest: `https://piston-meta.mojang.com/mc/game/version_manifest_v2.json`
- minecraft.wiki (JSON formats and authentication)
- Reference code: Prism Launcher, HMCL, portablemc, minecraft-launcher-lib
- Tauri v2: https://v2.tauri.app
