# Modrinth fixtures

`licht-modpack.json` reproduces the per-instance index that Licht v0.2.1–v0.2.3
wrote before Modrinth indexes moved to SQLite: the pack's `modrinth.index.json`,
pretty-printed with sorted keys. Its entries are synthetic: every file is the
11-byte text `hello world`, so tests can place them on disk and verify the
legacy import without network access. The server-only file is never stored.
