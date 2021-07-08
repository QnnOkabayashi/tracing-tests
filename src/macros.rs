#[macro_export]
macro_rules! alarm {
    ($($arg:tt)*) => {{
        let err = format!($($arg)*);
        // TODO: figure out how to get this to print the same as what
        // `tracing::error` generates.
        eprintln!("ALARM: {}", err);
        use tracing;
        tracing::error!(message = err.as_str());
    }};
}
