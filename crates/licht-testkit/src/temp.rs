use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// An empty folder under the system temp directory, removed with everything in it
/// when dropped.
///
/// The folder is named `licht-<label>-<process id>-<counter>`, so tests running in
/// parallel never share one and a leftover folder says which test made it.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(label: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "licht-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // A crashed run of an earlier process with the same id may have left it behind.
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the temp directory should be created");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::TempDir;

    #[test]
    fn creates_an_empty_labelled_folder() {
        let dir = TempDir::new("testkit-empty");

        assert!(dir.path().is_dir());
        assert!(dir.path().starts_with(std::env::temp_dir()));
        let name = dir.path().file_name().and_then(|name| name.to_str());
        assert!(
            name.is_some_and(|name| name.starts_with("licht-testkit-empty-")),
            "unexpected name: {name:?}"
        );
        assert_eq!(fs::read_dir(dir.path()).expect("read").count(), 0);
    }

    #[test]
    fn gives_each_folder_a_unique_path() {
        let first = TempDir::new("testkit-same");
        let second = TempDir::new("testkit-same");

        assert_ne!(first.path(), second.path());
    }

    #[test]
    fn removes_the_folder_and_its_contents_when_dropped() {
        let dir = TempDir::new("testkit-drop");
        let path = dir.path().to_path_buf();
        fs::create_dir_all(path.join("nested")).expect("nested folder");
        fs::write(path.join("nested").join("file.txt"), b"data").expect("file");

        drop(dir);

        assert!(!path.exists());
    }
}
