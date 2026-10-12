//! Discover, validate, and install Java runtimes (Eclipse Temurin).

mod archive;
mod detect;
mod require;
mod temurin;

pub use detect::{
    JavaDetections, JavaPaths, JavaStatus, browse_start_directory, default_java_roots,
    detect_javas, detect_known_javas, discover_javas, java_detect_roots, java_installation_status,
    matching_java, parse_java_version, probe_java_major, runtime_java_paths, use_custom_java,
    validate_java,
};
pub use require::{required_java_major, required_runtime};
pub use temurin::{ADOPTIUM_API, install_temurin, installed_java};
