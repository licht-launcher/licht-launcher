# Java runtime

`licht install` downloads Eclipse Temurin from Adoptium and stores it under
`runtime/` in the data directory. `licht launch` uses that copy. It does not
download Java, and it does not search for a Java already installed on the
machine. `--java <executable>` skips Temurin on install and launch.

The game files still come from Mojang. Temurin is the runtime only.

## Which major

`javaVersion.majorVersion` in the version JSON wins. The component name in
that object (`jre-legacy`, `java-runtime-gamma`, `java-runtime-delta`,
`java-runtime-epsilon`) is Mojang's label for its own runtime packages.
Licht only reads the major; it does not download Mojang's Java.

Installed JSONs on the Windows checks:

| Version                     | `majorVersion` | Component in the JSON  |
| --------------------------- | -------------- | ---------------------- |
| 1.5.2, 1.7.2, 1.8.9, 1.12.2 | 8              | `jre-legacy`           |
| 1.20.1                      | 17             | `java-runtime-gamma`   |
| 1.21.11                     | 21             | `java-runtime-delta`   |
| 26.3                        | 25             | `java-runtime-epsilon` |

Without `javaVersion`, the release id decides: 1.17 is 16, 1.18 through
1.20.4 is 17, 1.20.5 and newer is 21, and every other id is 8. A snapshot id
such as `24w14a` does not match that pattern, so it falls back to 8 unless
the JSON carries `javaVersion`. 26.3 carries `majorVersion` 25, so the id
rule is not what selects its runtime.

## Where it is downloaded

Adoptium's API, JRE first and JDK when that major has no JRE:

```text
https://api.adoptium.net/v3/assets/latest/<major>/hotspot?architecture=<arch>&image_type=jre&os=<os>&vendor=eclipse
```

`os` is `windows`, `linux`, or `mac`. `arch` is `x64` or `x86`. The package
checksum is SHA-256. A mismatch deletes the archive and fails the install.
The archive is a `.zip` or a `.tar.gz`, extracted into the runtime folder,
then removed. The `java` binary is marked executable.

A second install of the same major reuses the folder. When several folders
match, the JRE ranks above the JDK, and the higher version number wins.

The folder name is `temurin<major>-<jre|jdk><version>-<os>_<arch>`. After the
Windows installs the data directory held:

```text
runtime/temurin8-jre8.0.504-win_x64
runtime/temurin17-jre17.0.20-win_x64
runtime/temurin25-jre25.0.4-win_x64
```

The executable is `bin/java.exe` on Windows and `bin/java` on Linux and macOS.
A folder name of empty, `.`, `..`, or one that contains a slash is rejected.

## Launch

Without `--java`, launch looks for `temurin<major>-...` for this machine and
fails if that folder or its `java` binary is missing.

With `--java`, the major comes from `java -version`. Java 8 prints `1.8.0_…`.
Java 9 and later print `21.0.7`. The text is read from stderr. That major is
only for the log codec. Launch does not check it against `javaVersion`.

Java 18 and newer logs are read as UTF-8. An older Java is read as the system
code page. Details are in the log section of
[COMPATIBILITY.md](../COMPATIBILITY.md).

## Discovery of system Javas

Discovery of a Java under `C:\Program Files\Eclipse Adoptium` and the other
usual roots is implemented and tested for the settings UI. The CLI install
path only uses Temurin under Licht's `runtime/` folder (or `--java`).
