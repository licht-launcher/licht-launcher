//! Modrinth `.mrpack` import, loader apply, and file install.

mod import;
mod index;
mod install;
mod loader;
mod overrides;

pub use index::mrpack_download_tasks;
pub use install::install_mrpack_files;
pub use loader::apply_mrpack_loader;

/// Resolved Modrinth index for one instance (`modrinth.index.json` contents).
pub const MODPACK_INDEX_FILE: &str = "licht-modpack.json";

/// Files at the instance root that a pack must never overwrite.
const PROTECTED_ROOT_NAMES: [&str; 3] = ["instance.json", ".instance.json.tmp", MODPACK_INDEX_FILE];
pub(crate) use import::import_mrpack_into;
pub(crate) use install::mrpack_download_client;
pub(crate) use loader::apply_mrpack_loader_with_context;
