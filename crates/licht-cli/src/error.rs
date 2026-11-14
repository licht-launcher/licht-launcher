use licht_core::CoreError;

pub fn report(error: &CoreError) {
    eprintln!("{error}");
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        eprintln!("  {cause}");
        source = cause.source();
    }
}
