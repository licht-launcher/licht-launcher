# Licht Launcher (Tauri v2)

## Context

- **Unofficial** Minecraft: Java Edition launcher, cross-platform (**Windows and Linux**), built with **Tauri v2**.
- Frontend: React + TypeScript + Vite, Tailwind CSS, shadcn/ui (Radix UI), Lucide, TanStack Router (file-based routes in `src/routes`, hash history), TanStack Query, TanStack Virtual. See `package.json` for exact dependencies. Backend: Rust.
- Display name: **Licht Launcher**. Repository/packages: `licht-launcher`. Binary/CLI: `licht`. Core crate: `licht-core`.
- The project owner is a senior TypeScript/Node developer, but has **LITTLE knowledge of Rust**.
- The full plan is in `docs/ROADMAP.md`. Work on **one task at a time**; explicitly requested maintenance or documentation tasks do not need a roadmap item.

## How to work with me

- Before any code, present a **short PLAN** (files that will be created/changed, approach, alternatives) and wait for my approval.
- **Ask** when there is a decision: new dependency, architecture change, public API between Rust and the frontend, security/authentication, any ambiguity. Items marked with 🛑 in the roadmap always require my approval. Do not decide on your own.
- One task = one branch = one small, focused PR. Never mix topics.
- **Do not write code beyond what the task asks.** No "for the future" abstractions, no extra features, no refactoring that was not requested.
- Follow the architecture below. If you think it should change, **propose it first**; do not change it.
- File edits inside an already approved plan do not need individual confirmation.
- If something in the task conflicts with this file or the roadmap, stop and tell me.

## Language

- Code, identifiers, comments, commit messages (Conventional Commits: `feat:`, `fix:`, `test:`, `chore:`...), UI text and PR descriptions in **English**.

## Architecture

- Cargo workspace:
  - `crates/licht-core`: all the logic (manifests, downloads, rules, Java, launch, auth). **No Tauri dependency.** Testable on its own.
  - `src-tauri`: thin layer. Only `#[tauri::command]` and events that call `licht-core`. **No business rules here.**
  - `src/`: frontend. No Minecraft logic; only UI and `invoke` calls. Structure: `src/routes` (route files), `src/components/ui` (shadcn/ui), `src/features/*` (screens and UI logic by feature), `src/lib` (utilities and `invoke` wrappers).
- Rust → frontend communication: **events** (e.g. download progress). Frontend → Rust: **typed commands**.
- Types shared between Rust and TS must be **generated with `ts-rs`**, never duplicated by hand.
- Errors: `thiserror` in the core, `anyhow` only at the edges. **No `unwrap()`** outside tests.
- Logs with `tracing`.
- Use **Tauri v2**. Do not use v1 APIs or examples.

## UI and design system (consistency over creativity)

- Use **only** the components in `src/components/ui` and the **theme tokens** (CSS variables). If a component is missing, propose adding it to the design system instead of creating a one-off style.
- **Forbidden:** inline styles, loose colors/sizes (e.g. `#3b82f6`, `w-[137px]`) outside the tokens, and any new UI library without my approval.
- Before creating a screen, **list the existing components** that will be used and which ones are missing.
- Accessibility: everything operable by keyboard, with visible focus and correct labels (Radix already helps; do not remove that).
- The frontend runs in **WebView2 (Windows)** and **WebKitGTK (Linux)**. Avoid `backdrop-filter`, blurs, large shadows, and complex animations. If you use a modern CSS feature, tell me so I can test it on both systems.
- Calls to Rust go through TanStack Query (cache, loading, and error). Progress events with throttling (~100 ms). Virtualize lists expected to exceed ~100 items.

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

## Releases

- For anything about releases, versions or codenames, read `docs/RELEASING.md`.

## Efficiency (token budget)

### Reading

- Locate with `rg`/`grep` first; read only the needed line ranges, not whole files.
- Never read in full: `target/`, `node_modules/`, `dist/`, `Cargo.lock`, `pnpm-lock.yaml`, `src/routeTree.gen.ts` and other generated files (e.g. `src-tauri/gen/`, `src-tauri/icons/`), `tests/fixtures/*`. For fixtures use `jq`, `head` or `rg` to extract only the fields you need.
- In `docs/ROADMAP.md`, read only "Open items" and the current item/phase.
- Read other `docs/**/*.md` only when the current task touches that topic. Do not re-read files already in context unless they changed.
- Keep exploration scoped to the current item. Use targeted searches to fill missing context; ask when a decision or requirement remains unclear.

### Commands

- While iterating use the cheapest check: `cargo check -q`, `cargo test -q -p licht-core <filter>`, `pnpm vitest run <file>`. Run the full suite (fmt, clippy, tests, lint, typecheck) only before the PR.
- Prefer supported quiet or summary options. If output needs filtering, use shell-compatible commands and preserve the original command's exit code. On failure, show the relevant error with enough context to diagnose it.
- Do not rerun the same command without a relevant change or new evidence that justifies it, such as a suspected transient failure.

### Responses

- Short and direct. Do not recap what I already know or repeat the approved plan.
- Show only diffs or changed snippets, never whole files back.
- Plans as short bullets: files, approach, 🛑 decisions. PR descriptions concise; "Rust concepts used" is mandatory but cover only what is new in that PR.

### Session

- One task per session, whether a roadmap item or explicitly requested maintenance. When done, suggest ending the session.
- If the context is getting large, suggest compacting before continuing.
