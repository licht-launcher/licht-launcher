# Proposal: CurseForge key in official builds

> Status: **proposal, not implemented.** 🛑 Security and terms decision for the
> project owner. Nothing below changes code until the decisions at the end are made.

Today Licht reads the CurseForge key only from `LICHT_CURSEFORGE_API_KEY` (process
environment or `.env`). Release builds have no key, so regular users only see
Modrinth on Discover. This document proposes how official builds could ship a key.

## 1. What CurseForge's terms say

Sources (read 2026-11-10):

- [CurseForge 3rd Party API Terms and Conditions](https://support.curseforge.com/en/support/solutions/articles/9000207405)
  (page shows "Modified on: Wed, 14 Aug, 2024").
- [Applying for an API key](https://support.curseforge.com/en/support/solutions/articles/9000208346)
  (modified 3 Dec 2024) and the [3rd Party API folder](https://support.curseforge.com/en/support/solutions/folders/9000195851).
- [CurseForge REST API docs](https://docs.curseforge.com/rest-api/) and the
  [CurseForge for Studios console](https://console.curseforge.com/) (login required).

Key points:

- **One key per developer, not shareable.** §2.2: the key is "non-transferable and
  may not be shared with any third party"; disclosure is allowed only to employees
  under confidentiality. The terms have **no clause about embedding a key in
  distributed client software**. Shipping it inside a public binary is, strictly,
  making it available to anyone who extracts it.
- **No proxy to hide identity.** §3.1 forbids circumventing the API's security
  mechanisms and concealing identity, including through a proxy server or VPN.
- **No caching.** §3.1 also says developers may not "save or cache any data
  obtained through the API". This matters for Licht (see Risks).
- **Termination.** §8: termination for material breach after written notice with
  a 14-day cure period, or without cause for a specific app with 30 days' notice.
  §9: delete all copies of the key on termination.
- **No public rate limit.** §2.3 mentions a usage quota that may require a separate
  licensing agreement. The docs only state the pagination cap
  (`index + pageSize <= 10000`), which Licht already respects.
- **How keys are granted.** An application form, reviewed by Overwolf, which
  weighs how the app affects author earnings, CurseForge's servers/CDN, and how it
  "takes Authors' consent" for third-party distribution. Keys generated directly in
  the Studios console are meant for game developers; the docs say unused
  non-developer accounts may be deleted.

Conclusion: the terms neither allow nor forbid embedding explicitly. The launchers
below ship keys that Overwolf **issued for that specific launcher**. Licht should do
the same: apply for a Licht-specific application key, state in the application that
it will be embedded in open-source desktop builds, and keep Overwolf's written
answer. A personal or console key must not be embedded.

## 2. How other open-source launchers do it

| Launcher | Mechanism | User override | Source |
| --- | --- | --- | --- |
| Prism Launcher | Key committed in `CMakeLists.txt` as the cache variable `Launcher_CURSEFORGE_API_KEY`; comment says it "was issued specifically for Prism Launcher" and forks must change it | Yes, Settings → APIs → "CurseForge Core API" | [CMakeLists.txt](https://raw.githubusercontent.com/PrismLauncher/PrismLauncher/develop/CMakeLists.txt), [APIs help page](https://prismlauncher.org/wiki/help-pages/APIs) |
| ATLauncher | Key committed as a constant in `Constants.java`; README says forks must apply for their own | No | [Constants.java](https://github.com/ATLauncher/ATLauncher/blob/master/src/main/java/com/atlauncher/constants/Constants.java), [README](https://github.com/ATLauncher/ATLauncher) |
| CurseForge.ApiProxy | Self-hosted proxy that keeps a key server-side; its README says not to use it with a unique application key because the terms forbid proxy access | n/a | [repository](https://github.com/CurseForgeCommunity/CurseForge.ApiProxy) |

GDLauncher was not confirmed; its archived README only says private keys are
stored outside the repository.

Takeaways: the established launchers accept that the key is public in practice and
rely on it being issued to them. Licht can improve on one detail by keeping the key
**out of the repository** (CI secret), so forks and local builds never inherit it by
accident.

## 3. Proposed mechanism

Same shape as the Microsoft client ID in
[`crates/licht-core/src/auth/client_id.rs`](../../crates/licht-core/src/auth/client_id.rs)
(runtime env → embedded value), but the embedded value comes from a CI secret
instead of the source.

### Core (`crates/licht-core/src/modpacks/curseforge/mod.rs`)

```rust
/// Key baked in by the official release workflow. `None` in local and fork builds.
const EMBEDDED_CURSEFORGE_API_KEY: Option<&str> = option_env!("LICHT_CURSEFORGE_API_KEY_EMBED");

pub fn curseforge_api_key() -> Result<String> {
    // 1. Runtime env / .env (developers, forks, testing a new key).
    if let Ok(value) = std::env::var(CURSEFORGE_API_KEY_ENV)
        && !value.trim().is_empty()
    {
        return Ok(value.trim().to_string());
    }
    // 2. Official build key.
    match EMBEDDED_CURSEFORGE_API_KEY.map(str::trim) {
        Some(key) if !key.is_empty() => Ok(key.to_string()),
        _ => Err(CoreError::CurseforgeKeyMissing),
    }
}
```

- **A separate build-time name** (`LICHT_CURSEFORGE_API_KEY_EMBED`, not the runtime
  `LICHT_CURSEFORGE_API_KEY`), so a developer who has the runtime key in their shell
  does not bake it into a local build by accident.
- `option_env!` reads the variable when the crate is compiled. Cargo records it and
  rebuilds when it changes (to confirm on the pinned toolchain during
  implementation). `src-tauri/src/commands/shell.rs` already uses `option_env!` for
  `LICHT_BUILD_REVISION`.
- Callers do not change: search, details, import and CDN downloads already go
  through `curseforge_api_key()`, and `available_modpack_sources` (licht-launcher/licht-launcher#212)
  starts offering CurseForge automatically.
- **Never logged.** The key stays only in the `x-api-key` header. No new `tracing`
  field, error detail, `Debug` output or "About" text contains it. At most, a log
  line may say which source was used (`env` / `embedded`), never the value.
- **Tests** cover the order (env wins, embedded fallback, missing → error) through a
  small helper that takes the embedded value as a parameter, because a test cannot
  change a compile-time constant. No test uses a real key or the network.

### Workflows

- [`.github/workflows/release.yml`](../../.github/workflows/release.yml), step
  **Build Tauri application**: add
  `LICHT_CURSEFORGE_API_KEY_EMBED: ${{ secrets.LICHT_CURSEFORGE_API_KEY }}` next to
  the Tauri signing secrets. Optionally move the `build` job into a GitHub
  Environment (e.g. `release`) restricted to `v*` tags, so the secret is
  readable only by tag builds.
- [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) and
  [`.github/workflows/e2e.yml`](../../.github/workflows/e2e.yml): **no change**. CI and
  PR builds stay keyless; fork PRs never receive secrets anyway.
- **Build cache:** `release.yml` restores the Rust cache with `save-if: "false"`, so
  compiled objects that contain the key are not saved. Keep it that way, and never
  upload `target/` as an artifact from the release job.

### Docs

- Update [`CURSEFORGE.md`](CURSEFORGE.md) "Key" and "Terms": official builds
  embed a Licht-issued key; forks must apply for their own and must not copy it
  from a release binary; runtime env still overrides. Mention single-quoting the
  key in `.env` (`$2a$10$…` is expanded by dotenvy otherwise).
- Fix the stale path in [`.env.example`](../../.env.example)
  (`docs/modpacks/CURSEFORGE.md` → `docs/integrations/CURSEFORGE.md`).
- [`RELEASING.md`](../contributing/RELEASING.md): add `LICHT_CURSEFORGE_API_KEY` to the
  required repository secrets, with rotation steps.

Expected size: one focused PR touching about five files (core function + tests,
`release.yml`, three docs).

## 4. Risks

- **The key can be extracted.** Anyone can find it in the binary with `strings`
  (CurseForge keys have a recognizable `$2a$10$` prefix). XOR or similar obfuscation
  only raises the bar slightly; Prism and ATLauncher do not bother. Abuse by third
  parties is attributed to Licht's key and could get it revoked. Mitigation:
  application key issued to Licht, Overwolf aware of the distribution model,
  monitoring via the Studios console if available.
- **Rotation needs a release.** A new key only reaches users through a new build.
  The Tauri updater makes that fast for installed builds; portable builds update
  only when users download again. Rotation steps: add the new key to the secret, tag
  a patch release, then ask Overwolf to revoke the old one.
- **Revocation.** All official builds lose CurseForge at once.
  - Discover (with licht-launcher/licht-launcher#212): CurseForge is still "offered"
    because a key exists, so each search shows the quiet notice and keeps Modrinth
    results. Possible follow-up: remember a rejected key for the session and stop
    offering CurseForge.
  - Already imported CurseForge instances: `install_curseforge_files` needs the key
    for CDN downloads, so installing or repairing them fails until a new build or a
    runtime key. Files already on disk keep working.
- **Caching clause (§3.1).** Licht stores the resolved file list in
  `licht-curseforge.json` and keeps API responses in TanStack Query's memory cache.
  Ask Overwolf in the application whether this counts as caching under §3.1.
- **Secret exposure in CI.** A malicious change to `release.yml` merged into `main`
  could print the secret. GitHub masks secrets in logs, and branch protection plus
  an Environment limited to tags reduce the risk.
- **Forks.** Forks build without the key (same as today) and cannot inherit it from
  the source. They could still copy it out of an official binary; the docs say
  they must not.

## Decisions for the owner

1. **Apply for a Licht application key** through the CurseForge form, stating that
   it will be embedded in open-source desktop builds? Or not ship a key at all
   (status quo: Modrinth only, CurseForge for users who set their own key)?
2. **Ask Overwolf** explicitly about (a) embedding in public binaries and
   (b) the §3.1 caching clause vs `licht-curseforge.json` and in-memory caches,
   before the first release with the key?
3. **Accept that the key is extractable**, or add light obfuscation (no real
   security, more code)?
4. **GitHub Environment** restricted to tags for the release secret, or a plain
   repository secret like the Tauri signing key?
5. **User override in the UI** (like Prism's Settings → APIs), or keep the env/.env
   override only? A UI override would be a new public command between Rust and the
   frontend and would store the key in the OS keyring.
6. **Revoked-key behaviour:** keep the quiet notice per search, or stop offering
   CurseForge for the session after the first rejected key (small follow-up PR)?
