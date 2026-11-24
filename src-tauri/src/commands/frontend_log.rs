//! Errors the interface reports, written to the launcher log.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use tauri::State;

const MAX_MESSAGE_CHARS: usize = 8_000;
const REPORTS_PER_WINDOW: u32 = 20;
const RATE_WINDOW: Duration = Duration::from_secs(60);

/// Where the interface caught the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendErrorSource {
    Render,
    Window,
    Promise,
}

impl FrontendErrorSource {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "render" => Some(Self::Render),
            "window" => Some(Self::Window),
            "promise" => Some(Self::Promise),
            _ => None,
        }
    }
}

/// Caps how many interface errors reach the log, so a render loop cannot flood it.
#[derive(Default)]
pub struct FrontendLogLimiter {
    window: Mutex<Option<RateWindow>>,
}

struct RateWindow {
    started: Instant,
    accepted: u32,
    dropped: u32,
}

#[derive(Debug, PartialEq, Eq)]
enum Admission {
    Accept,
    DropFirst,
    Drop,
}

impl FrontendLogLimiter {
    fn admit(&self, now: Instant) -> Admission {
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);
        let current = match window.as_mut() {
            Some(current) if now.saturating_duration_since(current.started) < RATE_WINDOW => {
                current
            }
            _ => window.insert(RateWindow {
                started: now,
                accepted: 0,
                dropped: 0,
            }),
        };
        if current.accepted < REPORTS_PER_WINDOW {
            current.accepted += 1;
            return Admission::Accept;
        }
        current.dropped += 1;
        if current.dropped == 1 {
            Admission::DropFirst
        } else {
            Admission::Drop
        }
    }
}

#[tauri::command]
pub fn log_frontend_error(
    limiter: State<'_, FrontendLogLimiter>,
    source: String,
    message: String,
) -> Result<(), String> {
    let source = FrontendErrorSource::parse(&source)
        .ok_or_else(|| "Unknown interface error source".to_string())?;
    record(&limiter, Instant::now(), source, &message);
    Ok(())
}

fn record(limiter: &FrontendLogLimiter, now: Instant, source: FrontendErrorSource, message: &str) {
    match limiter.admit(now) {
        // Debug formatting escapes line breaks, so a report cannot forge other log lines.
        Admission::Accept => {
            tracing::error!(?source, message = ?truncated(message), "Interface error");
        }
        Admission::DropFirst => {
            tracing::warn!("Too many interface errors; dropping reports until the next minute");
        }
        Admission::Drop => {}
    }
}

fn truncated(message: &str) -> std::borrow::Cow<'_, str> {
    match message.char_indices().nth(MAX_MESSAGE_CHARS) {
        Some((end, _)) => format!("{} (truncated)", &message[..end]).into(),
        None => message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use licht_test_support::capture_logs;

    use super::{
        FrontendErrorSource, FrontendLogLimiter, MAX_MESSAGE_CHARS, RATE_WINDOW,
        REPORTS_PER_WINDOW, record,
    };

    #[test]
    fn only_known_sources_are_accepted() {
        assert_eq!(
            FrontendErrorSource::parse("render"),
            Some(FrontendErrorSource::Render)
        );
        assert_eq!(
            FrontendErrorSource::parse("window"),
            Some(FrontendErrorSource::Window)
        );
        assert_eq!(
            FrontendErrorSource::parse("promise"),
            Some(FrontendErrorSource::Promise)
        );
        assert_eq!(FrontendErrorSource::parse("Render"), None);
        assert_eq!(FrontendErrorSource::parse("network"), None);
    }

    #[test]
    fn reported_errors_are_logged_on_one_line_with_their_source() {
        let limiter = FrontendLogLimiter::default();

        let ((), logs) = capture_logs(|| {
            record(
                &limiter,
                Instant::now(),
                FrontendErrorSource::Render,
                "TypeError: x is undefined\n    at Panel (panel.tsx:3)",
            );
        });

        assert_eq!(logs.lines().count(), 1, "unexpected logs: {logs}");
        assert!(logs.contains("ERROR"), "unexpected logs: {logs}");
        assert!(logs.contains("Interface error"), "unexpected logs: {logs}");
        assert!(logs.contains("source=Render"), "unexpected logs: {logs}");
        assert!(
            logs.contains(r"TypeError: x is undefined\n    at Panel (panel.tsx:3)"),
            "unexpected logs: {logs}"
        );
    }

    #[test]
    fn long_messages_are_truncated() {
        let limiter = FrontendLogLimiter::default();
        let message = "é".repeat(MAX_MESSAGE_CHARS + 50);

        let ((), logs) = capture_logs(|| {
            record(
                &limiter,
                Instant::now(),
                FrontendErrorSource::Window,
                &message,
            );
        });

        assert!(logs.contains("(truncated)"), "unexpected logs: {logs}");
        assert!(logs.contains(&"é".repeat(MAX_MESSAGE_CHARS)));
        assert!(!logs.contains(&"é".repeat(MAX_MESSAGE_CHARS + 1)));
    }

    #[test]
    fn reports_over_the_limit_are_dropped_with_one_warning_until_the_window_ends() {
        let limiter = FrontendLogLimiter::default();
        let start = Instant::now();

        let ((), logs) = capture_logs(|| {
            for index in 0..REPORTS_PER_WINDOW + 5 {
                record(
                    &limiter,
                    start + Duration::from_millis(u64::from(index)),
                    FrontendErrorSource::Promise,
                    &format!("failure {index}"),
                );
            }
            record(
                &limiter,
                start + RATE_WINDOW,
                FrontendErrorSource::Promise,
                "after the window",
            );
        });

        let errors = logs.lines().filter(|line| line.contains("ERROR")).count();
        let warnings = logs.lines().filter(|line| line.contains("WARN")).count();
        assert_eq!(
            errors,
            REPORTS_PER_WINDOW as usize + 1,
            "unexpected logs: {logs}"
        );
        assert_eq!(warnings, 1, "unexpected logs: {logs}");
        assert!(logs.contains("Too many interface errors"));
        assert!(!logs.contains(&format!("failure {REPORTS_PER_WINDOW}")));
        assert!(logs.contains("after the window"));
    }
}
