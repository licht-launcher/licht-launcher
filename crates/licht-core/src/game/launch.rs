use std::collections::BTreeMap;
use std::path::Path;

use crate::settings::{GlobalLaunch, MIN_XMS_MB};
use crate::{
    Argument, ArgumentValue, GameArguments, LaunchEnvironment, Library, OsName, Result,
    SharedCache, Version, applicable_libraries, rules_allow,
};

/// Joined cache paths for this version and environment.
///
/// Windows uses `;`. Linux and macOS use `:`. The client jar is last.
/// A library whose Maven classifier starts with `natives-` is left out,
/// because those jars are extracted. An older library can list natives in a
/// separate map; its main jar stays, and the classifier jars are not added.
/// The files do not have to exist yet. These are the paths the cache uses.
pub fn classpath(
    cache: &SharedCache,
    version_id: &str,
    version: &Version,
    environment: &LaunchEnvironment,
) -> Result<String> {
    let mut entries = Vec::new();
    for library in applicable_libraries(version, environment) {
        if is_extracted_native(library) {
            continue;
        }
        let Some(artifact) = library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artifact.as_ref())
        else {
            continue;
        };
        entries.push(cache.library(&artifact.path)?);
    }
    entries.push(cache.version_jar(version_id)?);

    let separator = match environment.os {
        OsName::Windows => ";",
        OsName::Linux | OsName::Osx => ":",
    };
    let mut joined = String::new();
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            joined.push_str(separator);
        }
        joined.push_str(&entry.display().to_string());
    }
    Ok(joined)
}

/// Java executable, JVM arguments, the main class, then game arguments.
///
/// Modern JSON already contains `-cp` and `${classpath}`. Old JSON only stores
/// the game arguments, so the JVM prefix the official launcher adds is inserted
/// here: `-Djava.library.path=${natives_directory}`, `-cp`, `${classpath}`.
/// Those old game arguments are one string, split on ASCII whitespace. This is
/// not a shell.
///
/// `${classpath}` is always the string from [`classpath`], even when `values`
/// also has that key. Every other `${name}` is replaced from `values`. A name
/// that is missing stays written as `${name}`.
pub fn launch_command(
    java: &Path,
    cache: &SharedCache,
    version_id: &str,
    version: &Version,
    environment: &LaunchEnvironment,
    values: &BTreeMap<String, String>,
) -> Result<Vec<String>> {
    let classpath = classpath(cache, version_id, version, environment)?;
    let mut command = Vec::new();
    command.push(java.display().to_string());

    match &version.arguments {
        GameArguments::Modern { jvm, game } => {
            command.extend(evaluated_arguments(jvm, environment, values, &classpath));
            command.push(version.main_class.clone());
            command.extend(evaluated_arguments(game, environment, values, &classpath));
        }
        GameArguments::Legacy(game) => {
            command.push(substitute(
                "-Djava.library.path=${natives_directory}",
                values,
                &classpath,
            ));
            command.push("-cp".to_string());
            command.push(classpath.clone());
            command.push(version.main_class.clone());
            for token in game.split_ascii_whitespace() {
                command.push(substitute(token, values, &classpath));
            }
        }
    }

    Ok(command)
}

/// Inserts the global memory and JVM arguments immediately before the main class.
/// Fullscreen appends `--fullscreen` and leaves width and height out. Otherwise
/// `--width` and `--height` are appended only when they are set.
pub fn apply_global_launch(command: &mut Vec<String>, main_class: &str, global: &GlobalLaunch) {
    if let Some(index) = command.iter().position(|part| part == main_class) {
        let mut jvm = Vec::new();
        jvm.push(format!("-Xms{MIN_XMS_MB}M"));
        jvm.push(format!("-Xmx{}M", global.max_memory_mb));
        jvm.extend(global.jvm_arguments.iter().cloned());
        for (offset, argument) in jvm.into_iter().enumerate() {
            command.insert(index + offset, argument);
        }
    }
    if global.fullscreen {
        command.push("--fullscreen".to_string());
        return;
    }
    if let Some(width) = global.width {
        command.push("--width".to_string());
        command.push(width.to_string());
    }
    if let Some(height) = global.height {
        command.push("--height".to_string());
        command.push(height.to_string());
    }
}

fn is_extracted_native(library: &Library) -> bool {
    library
        .name
        .split(':')
        .nth(3)
        .is_some_and(|classifier| classifier.starts_with("natives-"))
}

fn evaluated_arguments(
    arguments: &[Argument],
    environment: &LaunchEnvironment,
    values: &BTreeMap<String, String>,
    classpath: &str,
) -> Vec<String> {
    let mut evaluated = Vec::new();
    for argument in arguments {
        match argument {
            Argument::Literal(value) => evaluated.push(substitute(value, values, classpath)),
            Argument::Conditional { rules, value } => {
                if !rules_allow(Some(rules), environment) {
                    continue;
                }
                match value {
                    ArgumentValue::Single(value) => {
                        evaluated.push(substitute(value, values, classpath));
                    }
                    ArgumentValue::Many(parts) => {
                        for part in parts {
                            evaluated.push(substitute(part, values, classpath));
                        }
                    }
                }
            }
        }
    }
    evaluated
}

fn substitute(input: &str, values: &BTreeMap<String, String>, classpath: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    while let Some((before, after_open)) = rest.split_once("${") {
        output.push_str(before);
        let Some((name, after)) = after_open.split_once('}') else {
            output.push_str("${");
            output.push_str(after_open);
            return output;
        };
        if name == "classpath" {
            output.push_str(classpath);
        } else if let Some(value) = values.get(name) {
            output.push_str(value);
        } else {
            output.push_str("${");
            output.push_str(name);
            output.push('}');
        }
        rest = after;
    }
    output.push_str(rest);
    library_path_root(output, values)
}

/// 26.3 asks for `${natives_directory}/java`. The DLLs live in the version folder.
fn library_path_root(value: String, values: &BTreeMap<String, String>) -> String {
    let Some(natives) = values.get("natives_directory") else {
        return value;
    };
    let prefix = format!("-Djava.library.path={natives}");
    if value == format!("{prefix}/java") || value == format!("{prefix}\\java") {
        return prefix;
    }
    value
}
