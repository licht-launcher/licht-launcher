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
| Client ID    | Runtime env `LICHT_MSA_CLIENT_ID` — never committed                    |
| Token store  | OS keyring (Credential Manager / Secret Service); never logs or files  |

## Azure app registration

1. Azure Portal → Microsoft Entra ID → App registrations → New registration.
2. Name: `Licht Launcher` (do not use Mojang / Minecraft / Microsoft / Xbox in the name).
3. Supported account types: **Personal Microsoft accounts only**.
4. Redirect URI: leave empty for device code (or public client / `http://localhost` if the portal requires one).
5. After create: Authentication → Advanced → **Allow public client flows** = **Yes**.
6. Do **not** create a client secret.
7. Copy **Application (client) ID** into a local env var (see below). Directory (tenant) ID is only for the Mojang form.

## Mojang allowlist

New client IDs get HTTP 403 (`Invalid app registration`) from
`api.minecraftservices.com` until Mojang allow-lists them.

1. Open https://aka.ms/mce-reviewappid
2. Choose **new app**
3. Submit Application (client) ID, Directory (tenant) ID, contact email (same as Azure), and a short justification
4. Submit once; reviews are often weekly

Until approval, device-code and Xbox steps may succeed and fail only at Minecraft Services.

## Local configuration

Copy [`.env.example`](../../.env.example) to `.env` in the **repository root** (gitignored):

```text
LICHT_MSA_CLIENT_ID=<application-client-id>
```

On startup, Licht loads `.env` from the current working directory and from the
directory of the executable. Values already set in the OS/shell environment are
not overwritten.

You can also set `LICHT_MSA_CLIENT_ID` as a user/system environment variable.
The value must not appear in the repository, commit messages, or logs.

## Flow (implemented in later items)

1. Device code → user completes `microsoft.com/link`
2. Xbox Live user authenticate (`RpsTicket: d=<msa_token>`)
3. XSTS for `rp://api.minecraftservices.com/`
4. `login_with_xbox` → Minecraft access token
5. Entitlements + profile
6. Tokens in the OS keyring; refresh via `offline_access`
