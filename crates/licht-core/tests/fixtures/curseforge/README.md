# CurseForge fixtures

`licht-curseforge.json` reproduces the per-instance index that Licht v0.2.1–v0.2.3
wrote before CurseForge indexes moved to SQLite. Its entries are synthetic: both
files are the 11-byte text `hello world`, so tests can place them on disk and
verify the legacy import without network access.
