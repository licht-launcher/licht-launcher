//! Capturing logs to assert on them.

use std::io::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};

use tracing::Dispatch;
use tracing::dispatcher::DefaultGuard;
use tracing_subscriber::EnvFilter;

/// Captures what this thread logs, formatted as the launcher writes it but without
/// timestamps or colors, until the capture is dropped.
///
/// Async tests must run on a current-thread runtime (the `#[tokio::test]` default)
/// so that spawned tasks log on the captured thread.
pub struct LogCapture {
    output: Arc<Mutex<Vec<u8>>>,
    _default: DefaultGuard,
    // While only one dispatcher is registered, tracing-core computes callsite interest
    // from the default of whichever thread hits a callsite first, so a parallel test
    // could cache a callsite as disabled. A second registered dispatcher makes every
    // thread take this capture into account.
    _registered: Dispatch,
}

impl LogCapture {
    /// Captures INFO events and above, the launcher's default level.
    pub fn start() -> Self {
        Self::filtered(EnvFilter::new("info"))
    }

    /// Captures the events `filter` enables.
    pub fn filtered(filter: EnvFilter) -> Self {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer_output = Arc::clone(&output);
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(move || SharedWriter(Arc::clone(&writer_output)))
            .with_ansi(false)
            .without_time()
            .finish();
        Self {
            output,
            _default: tracing::subscriber::set_default(subscriber),
            _registered: Dispatch::new(tracing::subscriber::NoSubscriber::default()),
        }
    }

    /// Everything captured so far.
    ///
    /// # Panics
    ///
    /// If the logs are not valid UTF-8, which would mean the formatter is broken.
    pub fn text(&self) -> String {
        let bytes = self
            .output
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        String::from_utf8(bytes).expect("captured logs should be valid UTF-8")
    }
}

struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::EnvFilter;

    use super::LogCapture;

    #[test]
    fn captures_events_until_dropped() {
        let capture = LogCapture::start();
        tracing::debug!("hidden by the default level");
        tracing::warn!(attempt = 2, "visible event");
        let logs = capture.text();
        drop(capture);
        tracing::warn!("after the capture");

        assert_eq!(logs.lines().count(), 1, "unexpected logs: {logs}");
        assert!(logs.contains("WARN"), "unexpected logs: {logs}");
        assert!(
            logs.contains("visible event attempt=2"),
            "unexpected logs: {logs}"
        );
    }

    #[test]
    fn filtered_captures_follow_the_filter() {
        let capture = LogCapture::filtered(EnvFilter::new("debug"));
        tracing::debug!("debug event");
        tracing::trace!("trace event");

        let logs = capture.text();
        assert!(logs.contains("debug event"), "unexpected logs: {logs}");
        assert!(!logs.contains("trace event"), "unexpected logs: {logs}");
    }
}
