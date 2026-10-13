# Forge and NeoForge installer fixtures

Only JSON metadata is stored here, extracted from official installers with its
values preserved. Trailing whitespace and line endings are normalized for Git.
No Minecraft binaries, processor jars, or resources outside the JSON metadata are
redistributed. Automated tests read these fixtures without network access.

| Fixture prefix | Installer source |
| --- | --- |
| `forge-legacy` | `https://maven.minecraftforge.net/net/minecraftforge/forge/1.12.2-14.23.5.2860/forge-1.12.2-14.23.5.2860-installer.jar` |
| `forge-modern` | `https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.3.0/forge-1.20.1-47.3.0-installer.jar` |
| `neoforge` | `https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.80/neoforge-21.1.80-installer.jar` |

Installers are checked against the `.sha1` sidecar before fixture extraction.
