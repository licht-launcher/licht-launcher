# Privacy Policy

Last updated: 2026-10-15

## Scope and contact

This policy describes the official Licht Launcher desktop application and CLI
distributed from [this repository](https://github.com/mateusneiva/licht-launcher).
The project is maintained by Mateus Neiva
([@mateusneiva](https://github.com/mateusneiva)). For privacy questions or
requests concerning information held by the maintainer, contact
[mateus.fneiva@gmail.com](mailto:mateus.fneiva@gmail.com).

**Unofficial project, not affiliated with Mojang Studios or Microsoft.**

Licht Launcher does not include product analytics, advertising, automatic
crash-report uploads, or cloud synchronization of your instances. It does make
the functional network requests described below. Minecraft, Java, mods, your
browser, and operating-system components have their own privacy practices.

## Data used and stored locally

| Data | Purpose and storage |
| --- | --- |
| Microsoft/Minecraft session | Refresh tokens, Minecraft access tokens, Minecraft profile name and UUID, Xbox identifier, and token expiry are stored in the OS keyring: Windows Credential Manager or Linux Secret Service. Short-lived authentication information is also processed in memory. |
| Remembered accounts | Account UUIDs are recorded in a local SQLite database so the launcher can list stored accounts. The active-account reference is stored in the OS keyring. |
| Settings | Offline username, theme, window and memory settings, Java paths, download settings, and selected data directory are stored in local settings files. These files are not encrypted by the launcher. |
| Instances and downloads | Instance names and configuration, last-played information, game files, Java runtimes, installed content, and metadata caches are stored in the configured launcher/instance directories. |
| UI and update preferences | The WebView's local/session storage holds UI preferences, update-check state, available/deferred update information, and related local state. |
| Local system information | OS, architecture, installed Java information, and total memory are used to select compatible files and offer settings. Java detection inspects configured/local locations. |

Licht Launcher does not ask for or store your Microsoft password. Sign-in takes
place in your system browser on Microsoft's sites. The launcher opens a
temporary loopback callback on `127.0.0.1` to receive an authorization code and
uses authorization code + PKCE with the `XboxLive.signin offline_access` scopes.
Authentication tokens are exchanged with Microsoft, Xbox, and Minecraft
Services to obtain a profile, verify entitlements, and refresh a session.
Signing in does not send your Microsoft password or session to the maintainer.

## Network requests

Requests go from your device to the providers needed for the feature in use.
Providers can receive your IP address, request headers, requested resources,
and the feature-specific data described here. Redirects, artifact repositories,
and CDNs used by those providers may also receive download requests.

| Feature | Recipients and information sent |
| --- | --- |
| Microsoft sign-in and authenticated launch | Microsoft identity, Xbox Live/XSTS, and Minecraft Services receive authorization codes, tokens, and authentication requests as required by their APIs. Tokens may be refreshed when needed for launch. |
| Game versions and installation | Mojang's metadata and distribution servers receive requests for manifests, version metadata, libraries, assets, and game files. |
| Java installation | Eclipse Adoptium/Temurin and its download hosts, including GitHub, receive requests selecting Java versions, OS, and architecture and requests for runtime files. |
| Mod-loader installation | Fabric, Quilt, Forge, NeoForge, and repositories referenced by their installation metadata receive version, installer, and library requests for the selected loader. |
| Modpack discovery and import | Modrinth or CurseForge and their CDNs receive search terms, filters, project/version/file identifiers, and download requests. CurseForge requests use an API key when configured; the key is sent to CurseForge's API. |
| Installed-content identification | Modrinth and, when an API key is configured, CurseForge can receive file hashes/fingerprints, project lookups, or search terms derived from content filenames. These identifiers can reveal which mods or packs are installed. Hashing and archive inspection happen locally; the metadata lookup does not upload the archive itself. |
| Content images | Image hosts referenced by provider metadata receive requests when the UI loads project or modpack icons. |
| Launcher updates | GitHub Releases and its download hosts receive update-manifest and installer requests. Automatic checks are enabled by default and can be disabled in Settings. |

Content metadata lookups can run automatically when an instance is selected,
including a prefetch at startup, or when content is displayed or refreshed.
Cached results reduce repeated lookups. There is currently no dedicated switch
to disable these metadata lookups. Choosing an offline profile or disabling
automatic launcher-update checks does not disable all network requests.

Third-party policies and information:

- Microsoft, Xbox, Minecraft, and Windows components:
  [Microsoft Privacy Statement](https://privacy.microsoft.com/en-us/privacystatement).
- GitHub, GitHub Releases, and repository interactions:
  [GitHub General Privacy Statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).
- Modrinth API and downloads:
  [Modrinth Privacy Policy](https://modrinth.com/legal/privacy).
- CurseForge API and downloads:
  [Overwolf Platform Privacy Policy](https://legal.overwolf.com/docs/overwolf/platform/platform-privacy-policy).
- Eclipse Adoptium/Temurin:
  [Eclipse Foundation Privacy Policy](https://www.eclipse.org/legal/privacy.php).
- Mod-loader services: [Fabric](https://fabricmc.net/),
  [Quilt](https://quiltmc.org/en/),
  [Forge privacy information](https://files.minecraftforge.net/static/privacy.html),
  and [NeoForge](https://neoforged.net/). Consult the relevant operator for
  policies covering its API, repositories, or CDN; website or Discord-bot
  policies do not necessarily cover those services.

Licht Launcher does not control those providers' retention, processing
locations, or their own analytics. A provider's website/app policy may describe
features that Licht Launcher does not implement. Launching Minecraft or a mod
can cause additional network activity outside the launcher's control.

## Logs and information you submit

Launcher diagnostics are written to standard error. Game console output is
displayed in the launcher, and Minecraft or mods can write their own log and
crash files in instance directories. These logs are not automatically uploaded
by Licht Launcher. Logs can contain player names, local file paths, server
addresses, and information printed by third-party code. Review and redact
logs and screenshots before sharing them; never send passwords or tokens.

GitHub issues, pull requests, and comments are public. If you email the
maintainer, the maintainer receives your address, message, and any attachments
you choose to send. Correspondence is used to respond, investigate reports,
and maintain necessary records, rather than for advertising or sale of data.
Email is handled through Gmail, subject to
[Google's Privacy Policy](https://policies.google.com/privacy).

## Your controls

- Microsoft sign-in is optional. An offline profile does not authenticate you
  to online services or remove the need for the appropriate game license.
- Remove a Microsoft account through Settings to delete its stored launcher
  credentials from the OS keyring and forget its local account-list entry.
  This does not delete the Microsoft account. Microsoft account permissions
  can also be managed through Microsoft's account services.
- In **Settings > Updater**, disable **Check for updates automatically** to
  stop startup update checks. Manual checks and other features still make
  their own requests.
- Disable **Install deferred updates on restart** to prevent a previously
  deferred update from being automatically installed on the next launch.
- Avoid optional online features you do not want to use. For device-wide
  network restrictions, use your OS/network controls; the launcher does not
  currently provide a comprehensive offline/privacy toggle.

If you are a minor, follow the account and parental-consent requirements of
Microsoft and the other services you use. Ask a parent or guardian before
sharing personal information in public issues or by email. Licht Launcher
does not independently collect a date of birth or perform age verification.

## Retention and deletion

Local settings, caches, and instances remain until you remove them; cached
tokens can expire or be replaced by refreshed credentials. Local/session
storage has the lifetime provided by the WebView and is not a cloud backup.

To remove local data, first sign out/remove saved accounts in the launcher,
back up worlds and other files you want to keep, and close the launcher. Use
the application-directory location shown in Settings to identify the launcher
data, and check the separate instance directories and any custom locations
you selected. Delete only the data you intend to remove. Uninstalling the
executable alone may leave these files, WebView storage, or keyring credentials
behind. Remaining credentials can be removed through the OS credential manager
under the `app.licht.launcher` service. See [uninstallation](README.md#uninstall).

The maintainer cannot remotely erase data stored only on your device. Contact
the email above to request access, correction, or deletion of personal
information you submitted to the maintainer. Correspondence may be retained
as needed to handle the request, maintain relevant records, or meet applicable
legal obligations. Requests about data held by Microsoft, GitHub, mod providers,
or other third parties must be directed to those providers. Applicable privacy
rights and the right to contact a competent data-protection authority remain
unaffected.

## Changes

Policy changes will be published here with a revised date. Material changes
to the launcher's data handling should be documented alongside the relevant
release. Questions about this policy can be sent to the contact above.
