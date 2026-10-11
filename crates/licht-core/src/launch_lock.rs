use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::{CoreError, Result};

const LOCK_FILE: &str = ".licht-launch.json";

/// Process identity written while a game is running for an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchLock {
    pub pid: u32,
    pub start_time: u64,
}

pub fn launch_lock_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join(LOCK_FILE)
}

pub fn write_launch_lock(instance_dir: &Path, lock: &LaunchLock) -> Result<()> {
    let path = launch_lock_path(instance_dir);
    let json = serde_json::to_string_pretty(lock)?;
    fs::write(path, json)?;
    Ok(())
}

/// Reads start time for `pid` and writes the lock file.
pub fn write_launch_lock_for_pid(instance_dir: &Path, pid: u32) -> Result<()> {
    let start_time = process_start_time(pid).ok_or_else(|| {
        CoreError::Io(std::io::Error::other(format!(
            "could not read start time for pid {pid}"
        )))
    })?;
    write_launch_lock(instance_dir, &LaunchLock { pid, start_time })
}

pub fn clear_launch_lock(instance_dir: &Path) -> Result<()> {
    let path = launch_lock_path(instance_dir);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CoreError::Io(error)),
    }
}

pub fn read_launch_lock(instance_dir: &Path) -> Result<Option<LaunchLock>> {
    let path = launch_lock_path(instance_dir);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(Some(serde_json::from_str(&text)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(CoreError::Io(error)),
    }
}

pub fn process_start_time(pid: u32) -> Option<u64> {
    let mut system = System::new();
    let handle = Pid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    system.process(handle).map(|process| process.start_time())
}

pub fn process_matches_lock(pid: u32, start_time: u64) -> bool {
    process_start_time(pid) == Some(start_time)
}

pub fn kill_launch_process(pid: u32) -> Result<()> {
    let mut system = System::new();
    let handle = Pid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    let Some(process) = system.process(handle) else {
        return Ok(());
    };
    if !process.kill() {
        return Err(CoreError::Io(std::io::Error::other(format!(
            "could not kill pid {pid}"
        ))));
    }
    Ok(())
}

/// Folders whose lock file still matches a live process. Stale locks are removed.
pub fn list_running_launch_folders(instances_dir: &Path) -> Result<Vec<String>> {
    if !instances_dir.exists() {
        return Ok(Vec::new());
    }
    let mut folders = Vec::new();
    for entry in fs::read_dir(instances_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().into_owned();
        let dir = entry.path();
        let Some(lock) = read_launch_lock(&dir)? else {
            continue;
        };
        if process_matches_lock(lock.pid, lock.start_time) {
            folders.push(folder);
        } else {
            clear_launch_lock(&dir)?;
        }
    }
    folders.sort();
    Ok(folders)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_lock_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!("licht-launch-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        let lock = LaunchLock {
            pid: 42,
            start_time: 1_700_000_000,
        };
        write_launch_lock(&dir, &lock).expect("write");
        assert_eq!(read_launch_lock(&dir).expect("read"), Some(lock));
        clear_launch_lock(&dir).expect("clear");
        assert_eq!(read_launch_lock(&dir).expect("gone"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn current_process_matches_its_start_time() {
        let pid = std::process::id();
        let start = process_start_time(pid).expect("start time");
        assert!(process_matches_lock(pid, start));
        assert!(!process_matches_lock(pid, start.wrapping_add(9_001)));
    }
}
