use std::env;

use licht_core::CoreError;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

pub fn init(verbose: u8) -> licht_core::Result<()> {
    let filter = parse_filter(verbose, env_log_directives()?)?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .try_init()
        .map_err(|error| CoreError::Io(std::io::Error::other(error.to_string())))
}

fn env_log_directives() -> licht_core::Result<Option<String>> {
    match env::var("RUST_LOG") {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("RUST_LOG must contain valid Unicode: {error}"),
        ))),
    }
}

fn parse_filter(verbose: u8, directives: Option<String>) -> licht_core::Result<EnvFilter> {
    let directives = match directives {
        Some(value) => value,
        None => match verbose {
            0 => String::new(),
            1 => "debug".to_string(),
            _ => "trace".to_string(),
        },
    };
    EnvFilter::builder()
        .with_regex(false)
        .with_default_directive(LevelFilter::INFO.into())
        .parse(directives)
        .map_err(|error| {
            CoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid RUST_LOG filter: {error}"),
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::parse_filter;

    #[test]
    fn default_filter_accepts_an_empty_directive() {
        parse_filter(0, None).expect("default filter");
    }

    #[test]
    fn verbose_defaults_to_debug_without_rust_log() {
        parse_filter(1, None).expect("debug filter");
        parse_filter(2, None).expect("trace filter");
    }

    #[test]
    fn rust_log_overrides_verbose() {
        parse_filter(2, Some("warn".into())).expect("explicit filter");
    }

    #[test]
    fn invalid_directives_return_an_io_error() {
        let error = parse_filter(0, Some("licht=invalid".into())).expect_err("invalid filter");
        match error {
            licht_core::CoreError::Io(error) => {
                assert!(error.to_string().contains("invalid RUST_LOG filter"));
            }
            other => panic!("expected I/O error, got {other}"),
        }
    }
}
