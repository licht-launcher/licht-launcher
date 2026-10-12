//! Discover, validate, and install Java runtimes (Temurin and Mojang).

mod archive;
mod detect;
mod mojang;
mod require;
mod temurin;

pub use detect::{
    JavaDetections, JavaPaths, JavaStatus, browse_start_directory, default_java_roots,
    detect_javas, detect_known_javas, discover_javas, java_detect_roots, java_installation_status,
    matching_java, parse_java_version, probe_java_major, runtime_java_paths, validate_java,
};
pub use mojang::{
    JAVA_RUNTIME_INDEX_URL, JavaChoice, JavaRuntimeBuild, JavaRuntimeEntry, JavaRuntimeIndex,
    JavaRuntimeManifest, JavaRuntimeManifestRef, fetch_java_runtime_index, install_java,
    parse_java_runtime_index, parse_java_runtime_manifest, runtime_platform, select_runtime,
};
pub use require::{required_java_major, required_runtime};
pub use temurin::{ADOPTIUM_API, install_temurin, installed_java};
