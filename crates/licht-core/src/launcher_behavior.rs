use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct LauncherBehavior {
    running: HashSet<String>,
    minimized_for_games: bool,
}

impl LauncherBehavior {
    pub fn game_started(&mut self, folder: &str, minimize: bool) -> bool {
        if !self.running.insert(folder.to_string()) {
            return false;
        }
        self.minimized_for_games |= minimize;
        minimize
    }

    pub fn game_finished(&mut self, folder: &str, minimize: bool, restore: bool) -> bool {
        if !self.running.remove(folder) || !self.running.is_empty() {
            return false;
        }
        let should_restore = self.minimized_for_games && minimize && restore;
        self.minimized_for_games = false;
        should_restore
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_only_after_the_last_started_game_finishes() {
        let mut behavior = LauncherBehavior::default();
        assert!(behavior.game_started("first", true));
        assert!(!behavior.game_started("first", true));
        assert!(behavior.game_started("second", true));
        assert!(!behavior.game_finished("first", true, true));
        assert!(behavior.game_finished("second", true, true));
        assert!(!behavior.game_finished("second", true, true));
    }

    #[test]
    fn does_not_restore_without_an_automatic_minimize_or_for_a_failed_launch() {
        let mut behavior = LauncherBehavior::default();
        assert!(!behavior.game_finished("failed", true, true));
        assert!(!behavior.game_started("manual", false));
        assert!(!behavior.game_finished("manual", true, true));
    }

    #[test]
    fn respects_disabled_preferences_and_resets_between_sessions() {
        let mut behavior = LauncherBehavior::default();
        assert!(behavior.game_started("first", true));
        assert!(!behavior.game_finished("first", true, false));
        assert!(behavior.game_started("second", true));
        assert!(!behavior.game_finished("second", false, true));
        assert!(!behavior.game_started("third", false));
        assert!(!behavior.game_finished("third", true, true));
    }
}
