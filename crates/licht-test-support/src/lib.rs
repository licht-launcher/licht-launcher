//! Helpers shared by the workspace's tests. Only ever a dev-dependency.

use std::io::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};

use tracing::Dispatch;
use tracing::dispatcher::DefaultGuard;
use tracing_subscriber::EnvFilter;

/// Captures what this thread logs, formatted as the launcher writes it but without
/// timestamps or colors, until the capture is dropped.
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
    /// Captures events at the formatter's default level (INFO and above).
    pub fn start() -> Self {
        let output = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer(&output))
            .with_ansi(false)
            .without_time()
            .finish();
        Self::install(output, Dispatch::new(subscriber))
    }

    /// Captures the events `filter` enables.
    pub fn filtered(filter: EnvFilter) -> Self {
        let output = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(writer(&output))
            .with_ansi(false)
            .without_time()
            .finish();
        Self::install(output, Dispatch::new(subscriber))
    }

    fn install(output: Arc<Mutex<Vec<u8>>>, dispatch: Dispatch) -> Self {
        Self {
            output,
            _default: tracing::dispatcher::set_default(&dispatch),
            _registered: Dispatch::new(tracing::subscriber::NoSubscriber::default()),
        }
    }

    /// Everything captured so far.
    pub fn text(&self) -> String {
        let bytes = self.output.lock().unwrap_or_else(PoisonError::into_inner);
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// Runs `emit` and returns its value with everything it logged at the default level.
pub fn capture_logs<T>(emit: impl FnOnce() -> T) -> (T, String) {
    let capture = LogCapture::start();
    let value = emit();
    (value, capture.text())
}

fn writer(output: &Arc<Mutex<Vec<u8>>>) -> impl Fn() -> SharedWriter + use<> {
    let output = Arc::clone(output);
    move || SharedWriter(Arc::clone(&output))
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
    use super::{LogCapture, capture_logs};
    use tracing_subscriber::EnvFilter;

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
    fn capture_logs_returns_the_value_and_its_logs() {
        let (value, logs) = capture_logs(|| {
            tracing::info!("computing");
            42
        });

        assert_eq!(value, 42);
        assert!(logs.contains("INFO"), "unexpected logs: {logs}");
        assert!(logs.contains("computing"), "unexpected logs: {logs}");
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
