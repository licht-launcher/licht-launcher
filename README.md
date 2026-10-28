# Licht Launcher

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Licht Launcher icon" width="128">
</p>

<p align="center">
  <strong>A simple home for Minecraft: Java Edition.</strong><br>
  Your game versions, modpacks, and profiles in one place. Available for Windows and Linux.
</p>

<p align="center">
  <a href="#download"><img src="https://img.shields.io/badge/status-Early_Release-orange" alt="Early Release"></a>
  <a href="https://github.com/licht-launcher/licht-launcher/actions/workflows/ci.yml"><img src="https://github.com/licht-launcher/licht-launcher/actions/workflows/ci.yml/badge.svg?branch=main" alt="Build status"></a>
  <a href="https://github.com/licht-launcher/licht-launcher/releases/latest"><img src="https://img.shields.io/github/v/release/licht-launcher/licht-launcher?label=latest%20release" alt="Latest release"></a>
  <a href="https://github.com/licht-launcher/licht-launcher/releases"><img src="https://img.shields.io/github/downloads/licht-launcher/licht-launcher/total?label=downloads" alt="Downloads"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a>
</p>

<p align="center">
  <a href="https://licht-launcher.github.io/">Website</a> ·
  <a href="https://github.com/licht-launcher/licht-launcher/releases/latest">Download</a> ·
  <a href="https://github.com/licht-launcher/licht-launcher/issues">Report an issue</a>
</p>

> **Early Release** — Licht Launcher is in active development. Features and compatibility are still being refined, and you may encounter bugs. Your feedback helps shape the launcher.

<p align="center">
  <strong>Download 0.2.3 · Iris</strong><br>
  <a href="https://github.com/licht-launcher/licht-launcher/releases/download/v0.2.3/Licht-Launcher-0.2.3-Iris-windows-x64-setup.exe"><img src="https://img.shields.io/badge/Windows_x64-Download-blue?style=for-the-badge" alt="Download Licht Launcher 0.2.3 for Windows x64"></a>
  <a href="https://github.com/licht-launcher/licht-launcher/releases/download/v0.2.3/Licht-Launcher-0.2.3-Iris-linux-x64.AppImage"><img src="https://img.shields.io/badge/Linux_x64-Download_AppImage-orange?style=for-the-badge" alt="Download Licht Launcher 0.2.3 for Linux x64 (AppImage)"></a>
</p>

![Licht Launcher screenshot](.github/screenshot.png)

## Why Licht?

Licht is for players who want vanilla and modded Minecraft organized in a clean, focused interface. It adds built-in mod-loader selection and modpack discovery to the familiar install-and-play experience of the official launcher, with separate profiles and automatic Java setup in the same flow. Prism is a more established option with a broader toolkit; choose Licht if its straightforward Windows and Linux experience fits the way you play and you would like to help shape an Early Release.

## Make room for the way you play

- **Separate profiles:** keep different game versions and modpacks organized in their own folders.
- **Vanilla or modded:** choose Fabric, Quilt, Forge, or NeoForge when creating a profile.
- **Modpacks:** browse and import packs from Modrinth and CurseForge. [CurseForge requires additional setup](docs/integrations/CURSEFORGE.md).
- **Java handled for you:** the launcher downloads the Java version your game needs.
- **Your account:** sign in with Microsoft or use a local offline profile.
- **Built-in controls:** adjust game settings, manage installed content, and view game logs without leaving the launcher.
- **Updates:** check for new launcher versions from Settings and choose whether to enable automatic checks.

## Download

The buttons above download **0.2.3 · Iris** directly. You can also choose a package below or find the latest version and release notes on [GitHub Releases](https://github.com/licht-launcher/licht-launcher/releases/latest).

| Your system | Choose |
| --- | --- |
| Windows (64-bit) | [Download the `.exe` installer](https://github.com/licht-launcher/licht-launcher/releases/download/v0.2.3/Licht-Launcher-0.2.3-Iris-windows-x64-setup.exe) |
| Linux (64-bit) | [Download the `.AppImage`](https://github.com/licht-launcher/licht-launcher/releases/download/v0.2.3/Licht-Launcher-0.2.3-Iris-linux-x64.AppImage) |
| Debian, Ubuntu, and compatible Linux distributions | [Download the `.deb` package](https://github.com/licht-launcher/licht-launcher/releases/download/v0.2.3/Licht-Launcher-0.2.3-Iris-linux-x64.deb) |

Portable downloads are also available for Windows and Linux.

### Get started

1. Download and install Licht Launcher for your system.
2. Open Settings to sign in with Microsoft or choose an offline username.
3. Create a profile and choose a game version or modpack.
4. Let the launcher prepare the game files, then select **Play**.

Licht Launcher is free and open source. It does not include a Minecraft license; you need the appropriate game rights to play.

## Help and feedback

Found a problem or have an idea? [Open an issue](https://github.com/licht-launcher/licht-launcher/issues) with your launcher version, operating system, and what happened. Please remove private information from screenshots and logs before sharing them.

For private security reports, privacy questions, or other private enquiries, email [mateus.fneiva@gmail.com](mailto:mateus.fneiva@gmail.com).

## Privacy and trust

Microsoft sign-in opens in your browser. Licht Launcher does not ask for your Microsoft password or automatically upload game logs or crash reports.

Update downloads are verified with the launcher's signing key. Windows publisher signing is not yet available; see the [code signing policy](docs/CODE_SIGNING.md) for details.

Read the [Terms of Use](TERMS.md) and [Privacy Policy](PRIVACY.md) for information about accounts, downloads, and local data.

## Uninstall

- **Windows:** close the launcher and remove it through Windows Settings > Apps.
- **Linux `.deb`:** remove the package through your system's package manager.
- **AppImage or portable:** close the launcher and remove the executable and any shortcuts you created.

Uninstalling may leave your profiles, worlds, and saved accounts on the computer. See [how to remove local data](PRIVACY.md#retention-and-deletion) if you want to remove those too.

## For contributors

Want to help build Licht Launcher? Start with the [contribution guide](CONTRIBUTING.md) for development setup and testing. The [roadmap](docs/ROADMAP.md), [architecture](docs/internals/ARCHITECTURE.md), and [compatibility notes](docs/internals/COMPATIBILITY.md) describe the technical details and planned work.

## Support

Licht Launcher is free and open source. If you would like to support its development, you can [sponsor Mateus Neiva on GitHub](https://github.com/sponsors/mateusneiva) or buy a coffee on Ko-fi.

<a href="https://ko-fi.com/mateusneiva"><img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" height="36" alt="Buy me a coffee on Ko-fi"></a>

## License and credits

Licensed under the [MIT License](LICENSE). Created and maintained by Mateus Neiva.

**Unofficial project, not affiliated with Mojang Studios or Microsoft.**
