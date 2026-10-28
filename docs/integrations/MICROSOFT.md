# Microsoft authentication

Licht signs players in with personal Microsoft accounts, then Xbox Live and
Minecraft Services. Offline launch stays available when no Microsoft account is
active.

## Decisions

| Topic        | Choice                                                                 |
| ------------ | ---------------------------------------------------------------------- |
| OAuth flow   | Authorization code + PKCE (`consumers` tenant), system browser         |
| Redirect URI | Loopback `http://localhost` (any port; Licht binds a free local port)  |
| Scopes       | `XboxLive.signin offline_access`                                       |
| Client secret| None (public client)                                                   |
| Client ID    | Embedded in the binary (`EMBEDDED_MSA_CLIENT_ID`); optional `.env` override |
| Token store  | OS keyring (Credential Manager / Secret Service); never logs or files  |

## Azure app registration

1. Azure Portal → Microsoft Entra ID → App registrations → New registration.
2. Name: `Licht Launcher` (do not use Mojang / Minecraft / Microsoft / Xbox in the name).
3. Supported account types: **Personal Microsoft accounts only**.
4. Redirect URI platform: **Mobile and desktop applications** → add
   `http://localhost` (Microsoft accepts any port on this URI).
5. After create: Authentication → Advanced → **Allow public client flows** = **Yes**.
6. Do **not** create a client secret.
7. Copy **Application (client) ID** for the Mojang form. Directory (tenant) ID is only for that form.
   Official Licht builds ship the client ID embedded in the binary (public OAuth client; not a secret).

## Mojang allowlist

New client IDs get HTTP 403 (`Invalid app registration`) from
`api.minecraftservices.com` until Mojang allow-lists them.

1. Open https://aka.ms/mce-reviewappid
2. Choose **new app**
3. Submit Application (client) ID, Directory (tenant) ID, contact email (same as Azure), and a short justification
4. Submit once; reviews are often weekly

Until approval, browser OAuth and Xbox steps may succeed and fail only at Minecraft Services.

## Client ID in builds

Release and local builds use the embedded public client ID in
`crates/licht-core/src/auth/client_id.rs`. End users do **not** need an env var.

For forks or local experiments, optionally override with `.env` (see
[`.env.example`](../../.env.example)) or `LICHT_MSA_CLIENT_ID` in the shell.
On startup Licht loads `.env` from the working directory and next to the
executable; existing OS/shell values are not overwritten.

**Tokens** (refresh / access) must never appear in the repository, commit
messages, or logs — only the public client ID is embedded.

### Changing the embedded release client ID

Do this only when rotating the Azure app (compromise, lost tenant, new brand, etc.).

1. Create a **new** Entra app registration (same settings as above: personal
   accounts only, public client flows enabled, `http://localhost` redirect, no
   secret), or reuse another already allow-listed app you control.
2. Submit the **new** Application (client) ID to Mojang via
   https://aka.ms/mce-reviewappid (**new app**). Wait until
   `api.minecraftservices.com` accepts it (old ID keeps working until you stop
   shipping it).
3. Update `EMBEDDED_MSA_CLIENT_ID` in
   [`crates/licht-core/src/auth/client_id.rs`](../../crates/licht-core/src/auth/client_id.rs).
4. Ship a new Licht release that contains the rebuilt binary. Users must update
   the app; there is no server-side way to change an already installed client ID.
5. Optional cleanup: delete or disable the old Azure app when no supported Licht
   version still embeds it. Users on old builds will fail Microsoft sign-in until
   they upgrade.

For a one-off local test of a different ID without changing the release constant,
use `.env` / `LICHT_MSA_CLIENT_ID` (overrides the embedded value).

## Flow

1. Bind `127.0.0.1` on a free port; open the authorize URL in the system browser
   (PKCE S256 + `state`)
2. User signs in; Microsoft redirects to `http://localhost:<port>/?code=...`
3. Exchange code → MSA tokens
4. Xbox Live user authenticate (`RpsTicket: d=<msa_token>`)
5. XSTS for `rp://api.minecraftservices.com/`
6. `login_with_xbox` → Minecraft access token
7. Entitlements + profile
8. Tokens in the OS keyring; refresh via `offline_access`
