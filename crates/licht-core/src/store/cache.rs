use std::path::{Path, PathBuf};

use crate::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedCache {
    root: PathBuf,
}

impl SharedCache {
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn system() -> Result<Self> {
        let directories = directories::ProjectDirs::from("app.licht", "Licht", "Licht Launcher")
            .ok_or(CoreError::DataDirectory)?;
        Ok(Self::at(directories.data_dir()))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create(&self) -> Result<()> {
        for relative in [
            "assets/indexes",
            "assets/objects",
            "libraries",
            "natives",
            "runtime",
            "versions",
        ] {
            std::fs::create_dir_all(self.relative(relative)?)?;
        }
        Ok(())
    }

    pub fn asset_object(&self, hash: &str) -> Result<PathBuf> {
        let hash = hash.to_ascii_lowercase();
        if hash.len() != 40 || !hash.chars().all(|character| character.is_ascii_hexdigit()) {
            return Err(CoreError::CachePath);
        }
        Ok(self
            .root
            .join("assets")
            .join("objects")
            .join(&hash[..2])
            .join(hash))
    }

    pub fn asset_index(&self, id: &str) -> Result<PathBuf> {
        let name = single_component(id)?;
        Ok(self
            .root
            .join("assets")
            .join("indexes")
            .join(format!("{name}.json")))
    }

    /// `<cache>/assets/virtual/<id>`, shared by every instance of that index.
    pub fn virtual_assets_dir(&self, id: &str) -> Result<PathBuf> {
        Ok(self
            .root
            .join("assets")
            .join("virtual")
            .join(single_component(id)?))
    }

    pub fn natives_dir(&self, version_id: &str) -> Result<PathBuf> {
        Ok(self
            .root
            .join("natives")
            .join(single_component(version_id)?))
    }

    /// `<launcher>/instances`, the sibling of this data directory.
    pub fn instances_dir(&self) -> Result<PathBuf> {
        let launcher = self.root.parent().ok_or(CoreError::CachePath)?;
        Ok(launcher.join("instances"))
    }

    /// `<launcher>/instances/<id>`, the sibling of this data directory.
    ///
    /// LaunchWrapper versions keep saves in `<id>/.minecraft`. Other versions use `<id>` itself.
    pub fn instance_dir(&self, id: &str) -> Result<PathBuf> {
        Ok(self.instances_dir()?.join(single_component(id)?))
    }

    pub fn library(&self, artifact_path: &str) -> Result<PathBuf> {
        self.relative(&format!("libraries/{artifact_path}"))
    }

    pub fn version_jar(&self, id: &str) -> Result<PathBuf> {
        let name = single_component(id)?;
        Ok(self
            .root
            .join("versions")
            .join(&name)
            .join(format!("{name}.jar")))
    }

    pub fn runtime_dir(&self, component: &str, platform: &str) -> Result<PathBuf> {
        Ok(self
            .root
            .join("runtime")
            .join(single_component(component)?)
            .join(single_component(platform)?))
    }

    pub fn version_json(&self, id: &str) -> Result<PathBuf> {
        let name = single_component(id)?;
        Ok(self
            .root
            .join("versions")
            .join(&name)
            .join(format!("{name}.json")))
    }

    fn relative(&self, path: &str) -> Result<PathBuf> {
        if path.is_empty() {
            return Err(CoreError::CachePath);
        }
        let mut full = self.root.clone();
        for component in path.split(['/', '\\']) {
            if component.is_empty() || component == "." || component == ".." {
                return Err(CoreError::CachePath);
            }
            full.push(component);
        }
        Ok(full)
    }
}

fn single_component(name: &str) -> Result<String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(CoreError::CachePath);
    }
    Ok(name.to_string())
}
