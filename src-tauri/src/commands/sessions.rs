use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex as StdMutex;

use licht_core::{
    clear_launch_lock, kill_launch_process, list_running_launch_folders, process_matches_lock,
    read_launch_lock,
};
use tokio::task::AbortHandle;

use super::failure;

enum LaunchSession {
    Starting,
    Live { abort: AbortHandle },
    Orphan { pid: u32 },
}

/// Tracks which instance folders currently have a running game.
pub struct LaunchSessions {
    active: StdMutex<HashMap<String, LaunchSession>>,
}

/// Removes the folder from [`LaunchSessions`] when dropped.
pub struct LaunchGuard<'a> {
    sessions: &'a LaunchSessions,
    folder: String,
}

impl LaunchSessions {
    pub fn new() -> Self {
        Self {
            active: StdMutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, LaunchSession>>, String> {
        self.active
            .lock()
            .map_err(|_| "launch session lock is poisoned".to_string())
    }

    pub fn try_begin(&self, folder: &str) -> Result<LaunchGuard<'_>, String> {
        let mut active = self.lock()?;
        if active.contains_key(folder) {
            return Err(format!("instance \"{folder}\" is already running"));
        }
        active.insert(folder.to_string(), LaunchSession::Starting);
        Ok(LaunchGuard {
            sessions: self,
            folder: folder.to_string(),
        })
    }

    pub(super) fn attach_live(&self, folder: &str, abort: AbortHandle) -> Result<(), String> {
        let mut active = self.lock()?;
        match active.get_mut(folder) {
            Some(slot @ LaunchSession::Starting) => {
                *slot = LaunchSession::Live { abort };
                Ok(())
            }
            Some(_) => Err(format!("instance \"{folder}\" is already running")),
            None => Err(format!("instance \"{folder}\" has no launch session")),
        }
    }

    pub(super) fn reconcile(&self, instances_dir: &Path) -> Result<(), String> {
        let found = list_running_launch_folders(instances_dir).map_err(failure)?;
        let mut active = self.lock()?;
        for folder in found {
            if active.contains_key(&folder) {
                continue;
            }
            let dir = instances_dir.join(&folder);
            let Some(lock) = read_launch_lock(&dir).map_err(failure)? else {
                continue;
            };
            active.insert(folder, LaunchSession::Orphan { pid: lock.pid });
        }

        let stale: Vec<String> = active
            .iter()
            .filter_map(|(folder, session)| match session {
                LaunchSession::Orphan { .. } => {
                    let dir = instances_dir.join(folder);
                    match read_launch_lock(&dir) {
                        Ok(Some(lock)) if process_matches_lock(lock.pid, lock.start_time) => None,
                        _ => Some(folder.clone()),
                    }
                }
                _ => None,
            })
            .collect();
        for folder in stale {
            active.remove(&folder);
            let _ = clear_launch_lock(&instances_dir.join(&folder));
        }
        Ok(())
    }

    pub(super) fn list_running(&self, instances_dir: &Path) -> Result<Vec<String>, String> {
        self.reconcile(instances_dir)?;
        let active = self.lock()?;
        let mut folders: Vec<String> = active.keys().cloned().collect();
        folders.sort();
        Ok(folders)
    }

    pub(super) fn stop(&self, folder: &str, instance_dir: &Path) -> Result<(), String> {
        enum Action {
            Abort(AbortHandle),
            Kill(u32),
            Busy,
            Missing,
        }

        let action = {
            let mut active = self.lock()?;
            match active.get(folder) {
                Some(LaunchSession::Live { abort }) => Action::Abort(abort.clone()),
                Some(LaunchSession::Orphan { pid }) => {
                    let pid = *pid;
                    active.remove(folder);
                    Action::Kill(pid)
                }
                Some(LaunchSession::Starting) => Action::Busy,
                None => Action::Missing,
            }
        };

        match action {
            Action::Abort(abort) => {
                abort.abort();
                Ok(())
            }
            Action::Kill(pid) => {
                kill_launch_process(pid).map_err(failure)?;
                clear_launch_lock(instance_dir).map_err(failure)?;
                Ok(())
            }
            Action::Busy => Err(format!("instance \"{folder}\" is still starting")),
            Action::Missing => {
                if let Some(lock) = read_launch_lock(instance_dir).map_err(failure)? {
                    if process_matches_lock(lock.pid, lock.start_time) {
                        kill_launch_process(lock.pid).map_err(failure)?;
                    }
                    clear_launch_lock(instance_dir).map_err(failure)?;
                }
                Ok(())
            }
        }
    }
}

impl Drop for LaunchGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.sessions.active.lock() {
            active.remove(&self.folder);
        }
    }
}
