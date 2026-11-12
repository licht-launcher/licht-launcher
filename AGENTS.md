# Licht Launcher

Rules for anyone (human or agent) changing this repository. They are not
suggestions: code that breaks them does not get merged.

## Context

- **Unofficial** Minecraft: Java Edition launcher for **Windows and Linux**, built with **Tauri v2**.
- Backend: Rust (Cargo workspace). Frontend: React, strict TypeScript, Vite, Tailwind CSS, shadcn/ui (Radix UI), Lucide, TanStack Router (file-based routes in `src/routes`, hash history), TanStack Query and TanStack Virtual. `package.json` and `Cargo.toml` are the source of truth for versions.
- Names: display name **Licht Launcher**; repository/packages `licht-launcher`; binary/CLI `licht`; core crate `licht-core`.
- Pending work lives in `docs/ROADMAP.md`. Layers and boundaries are detailed in `docs/contributing/ARCHITECTURE.md`; setup and checks in `docs/contributing/DEVELOPMENT.md`.

## Workflow

- Before writing code, present a **short plan**: files to create/change, approach, alternatives considered, and how it will be tested. Wait for approval.
- **Ask first** for: new dependencies, architecture changes, the command/event API between Rust and the frontend, security or authentication, data stored on disk, and any ambiguity in the requirement. Roadmap items marked 🛑 always need approval.
- One task = one branch (`feat/...`, `fix/...`, `chore/...`) = one small, focused PR. Never mix topics.
- Do only what the task asks. No speculative abstractions, extra features or unrequested refactors. If you spot a problem outside the scope, report it instead of fixing it.
- If the task conflicts with this file, the architecture or the roadmap, stop and say so.
- Edits inside an approved plan do not need individual confirmation.

## Language

- Code, identifiers, comments, commit messages (Conventional Commits), PR descriptions and UI copy in **English**.

## Architecture

- `crates/licht-core`: all domain logic (manifests, downloads, rules, Java, launch, instances, auth, loaders, modpacks). **No Tauri dependency.** Testable on its own; it also builds the `licht` CLI.
- `src-tauri`: thin adapter. Only `#[tauri::command]` handlers, events and app wiring that delegate to `licht-core`. **No business rules.**
- `src/`: UI only, no Minecraft logic.
  - `src/routes`: route files that mount feature screens.
  - `src/features/<feature>`: screens, components, hooks and UI logic for one feature.
  - `src/components/ui`: shared design-system components.
  - `src/api`: the only place that talks to Tauri (typed `invoke` wrappers, payload parsing, events, window/platform helpers).
  - `src/lib`: generic helpers and the `ts-rs` types in `src/lib/generated`.
- Frontend → Rust: **typed commands**. Rust → frontend: **events** (progress throttled to ~100 ms).
- Types shared between Rust and TypeScript are **generated with `ts-rs`**, never written by hand.
- Use **Tauri v2** APIs only.
- If you believe the architecture should change, propose it; do not change it in passing.

## Code quality

### General

- Small, single-purpose functions and modules with clear names. Prefer simple, explicit code over clever code.
- Make invalid states unrepresentable: model domain data with precise types (enums, discriminated unions, newtypes) instead of loose strings and flags.
- Validate external input at the boundary (HTTP responses, files on disk, Tauri payloads, user input) and work with trusted types inside.
- Never swallow errors. Handle them or propagate them with context; the user must get an actionable message.
- No dead code, commented-out code, leftover debug output, or TODOs without an issue.
- Comments explain the non-obvious *why*, not the *what*.
- Match the patterns already in the codebase. Search for an existing helper before creating one.

### Rust

- Idiomatic, `clippy`-clean code (`-D warnings`). No `#[allow(...)]` without a comment justifying it.
- Errors: `thiserror` types in `licht-core`, with variants the caller can act on; `anyhow` only at the edges (`src-tauri`, CLI). **No `unwrap()`/`expect()`/`panic!` outside tests**, except for invariants that are documented at the call site.
- No `unsafe`. Async with `tokio`; never block the runtime (use `spawn_blocking` for blocking I/O or CPU-heavy work).
- Logging with `tracing`, with structured fields. Never log tokens, credentials or personal data.
- Keep the public API of each module minimal (`pub(crate)` by default).

### TypeScript and React

- `strict` TypeScript. No `any`, no non-null assertions to silence the compiler, no `@ts-ignore`/`@ts-expect-error`; use `unknown` and narrow.
- Data from Rust goes through TanStack Query (cache, loading and error states). Components never call `invoke` directly; they use `src/api`.
- Keep logic out of components: pure functions in `lib/`/`model/` and hooks in `hooks/` within the feature, so they can be tested in isolation.
- Virtualize lists expected to exceed ~100 items.
- Never disable lint rules (Biome) to make code pass.

## UI and design system

- Use **only** components from `src/components/ui` and **theme tokens** (CSS variables). If a component is missing, propose adding it to the design system instead of creating a one-off style.
- One folder per component: `<name>.tsx`, `<name>.styles.ts` (Tailwind classes), `<name>.test.tsx` and `index.ts`. Components added with the shadcn CLI must be moved into this layout (see `docs/contributing/ARCHITECTURE.md`).
- **Forbidden:** inline styles, arbitrary values outside the tokens (e.g. `#3b82f6`, `w-[137px]`), and new UI libraries without approval.
- Before building a screen, list the existing components it will use and the ones that are missing.
- Accessibility: everything operable by keyboard, with visible focus, correct roles and labels. Do not remove what Radix provides.
- The UI runs on **WebView2 (Windows)** and **WebKitGTK (Linux)**. Avoid `backdrop-filter`, blurs, large shadows and heavy animations. Flag any modern CSS feature so it can be checked on both platforms.

## Testing

Tests are part of the change, not a follow-up. A PR without tests for new behavior is incomplete.

- **Bugs:** write a failing test that reproduces the bug first, then fix it.
- **Test behavior, not implementation.** Assert on outputs, side effects and what the user sees; do not assert on private functions, internal state or call order unless that is the contract.
- **Determinism:** no network, no real Microsoft account, no dependency on the machine's clock, timezone, locale or file system layout. Use temporary directories, fake clocks and local test servers (`crates/licht-core/src/test_http.rs`).
- **Fixtures:** reuse the real Mojang/loader JSONs in `crates/licht-core/tests/fixtures` and the feature `fixtures/` folders. Only add third-party material whose redistribution is allowed, and document its source.
- Cover the unhappy paths: invalid input, corrupted or partial downloads, SHA1 mismatch, cancellation, missing files and permission errors.
- Test names describe the behavior (`rejects_a_file_with_a_wrong_sha1`, `"shows an error when the install fails"`).

### Rust

- Unit tests in `#[cfg(test)]` modules next to the code; cross-module flows in `crates/licht-core/tests`.
- Every public function of `licht-core` with logic has tests, including its error variants.
- `src-tauri` stays thin enough that it does not need logic tests; if a command needs one, the logic belongs in the core.

### Frontend

- Vitest + Testing Library. Query by role, label and text as a user would (`getByRole`, `getByLabelText`), drive interactions through events, and avoid test IDs unless there is no accessible alternative.
- Mock at the boundary (`src/api` or `@tauri-apps/*`), never the component under test or its children.
- Every component has at least one render test and one test per meaningful interaction or state (loading, empty, error, success).
- Pure functions and hooks get direct unit tests.
- End-to-end tests (`e2e/`, WebdriverIO) cover critical flows that do not need the network or an account.

## Quality gate (required before opening a PR)

- `pnpm check` must pass (or `pnpm check:web` / `pnpm check:rust` when only one side changed):
  - Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
  - Frontend: `pnpm lint`, `pnpm typecheck`, `pnpm test`.
- For build or packaging changes, also run the relevant build from `docs/contributing/DEVELOPMENT.md`.
- Never weaken, skip or delete a test to get green. If a check fails and you cannot fix it, say so in the PR.

## Pull requests

- Short title in Conventional Commits style.
- Description: what changed, why, how to test (commands actually run and their results), decisions made, and points that need review.
- Self-review the diff before opening it: scope, naming, tests, no leftovers.

## Domain rules

- Game data **always** comes from Mojang's servers. Never redistribute Minecraft files.
- Verify the **SHA1** (or the hash the source provides) of everything downloaded.
- No tokens or secrets in the repository or in logs. Tokens live in the **OS keyring**.
- Paths must work on Windows and Linux: use `std::path` and the `directories` crate; never build paths by concatenating strings.
- Changes to external requests, local storage or data handling must be reflected in `PRIVACY.md`.
- The project is **not official**: keep the "not affiliated with Mojang Studios or Microsoft" notice in the README. Do not use "Minecraft" or "Mojang" in the project, package or binary name, and do not use official logos or art.

## Releases

- For releases, versions or codenames, read `docs/contributing/RELEASING.md`.

## Agent efficiency

### Reading

- Locate with `rg` first; read only the line ranges you need.
- Never read in full: `target/`, `node_modules/`, `dist/`, `coverage/`, lockfiles, `src/routeTree.gen.ts`, `src/lib/generated/`, `src-tauri/gen/`, `src-tauri/icons/`, and fixtures. For fixtures use `jq`, `head` or `rg` to extract only the fields you need.
- In `docs/ROADMAP.md`, read only "Open items" and the current item. Read other `docs/**` only when the task touches that topic.

### Commands

- While iterating, use the cheapest check: `cargo check -q`, `cargo test -q -p licht-core <filter>`, `pnpm vitest run <file>`. Run the full gate before the PR.
- Preserve the exit code when filtering output, and show enough of a failure to diagnose it.
- Do not rerun a command without a relevant change or a suspected transient failure.

### Responses

- Short and direct. Show diffs or changed snippets, not whole files.
- Plans as short bullets: files, approach, tests, 🛑 decisions.
- One task per session; suggest compacting when the context gets large.
