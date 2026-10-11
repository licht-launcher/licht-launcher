# Microsoft authentication

Licht signs players in with personal Microsoft accounts, then Xbox Live and
Minecraft Services. Offline launch stays available when no Microsoft account is
active.

## Decisions

| Topic        | Choice                                                                 |
| ------------ | ---------------------------------------------------------------------- |
| OAuth flow   | Device code (`consumers` tenant)                                       |
| Scopes       | `XboxLive.signin offline_access`                                       |
| Client secret| None (public client)                                                   |
| Client ID    | Embedded in the binary (`EMBEDDED_MSA_CLIENT_ID`); optional `.env` override |
| Token store  | OS keyring (Credential Manager / Secret Service); never logs or files  |

## Azure app registration

1. Azure Portal → Microsoft Entra ID → App registrations → New registration.
2. Name: `Licht Launcher` (do not use Mojang / Minecraft / Microsoft / Xbox in the name).
3. Supported account types: **Personal Microsoft accounts only**.
4. Redirect URI: leave empty for device code (or public client / `http://localhost` if the portal requires one).
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

Until approval, device-code and Xbox steps may succeed and fail only at Minecraft Services.

## Client ID in builds

Release and local builds use the embedded public client ID in
`crates/licht-core/src/auth/client_id.rs`. End users do **not** need an env var.

For forks or local experiments, optionally override with `.env` (see
[`.env.example`](../../.env.example)) or `LICHT_MSA_CLIENT_ID` in the shell.
On startup Licht loads `.env` from the working directory and next to the
executable; existing OS/shell values are not overwritten.

**Tokens** (refresh / access) must never appear in the repository, commit
messages, or logs — only the public client ID is embedded.

## Flow (implemented in later items)

1. Device code → user completes `microsoft.com/link`
2. Xbox Live user authenticate (`RpsTicket: d=<msa_token>`)
3. XSTS for `rp://api.minecraftservices.com/`
4. `login_with_xbox` → Minecraft access token
5. Entitlements + profile
6. Tokens in the OS keyring; refresh via `offline_access`
