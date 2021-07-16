#[macro_export]
macro_rules! alarm {
    ($($arg:tt)*) => {{
        let err = format!($($arg)*);
        // TODO: figure out how to get this to print the same as what
        // `tracing::error` generates.
        eprintln!("[alarm]: {}", err);
        use tracing;
        tracing::error!("{}", err);
    }};
}

#[allow(unused_macros)]
macro_rules! tagged_event {
    ($logtag:ident, $event:ident, $($arg:tt)*) => {{
        use tracing;
        tracing::$event!(event_tag = u64::from(crate::subscriber::MyEventTag::$logtag), $($arg)*)
    }}
}

#[macro_export]
macro_rules! admin_error {
    ($($arg:tt)*) => { tagged_event!(AdminError, error, $($arg)*) }
}

#[macro_export]
macro_rules! admin_warn {
    ($($arg:tt)*) => { tagged_event!(AdminWarn, warn, $($arg)*) }
}

#[macro_export]
macro_rules! admin_info {
    ($($arg:tt)*) => { tagged_event!(AdminInfo, info, $($arg)*) }
}

#[macro_export]
macro_rules! request_error {
    ($($arg:tt)*) => { tagged_event!(RequestError, error, $($arg)*) }
}

#[macro_export]
macro_rules! request_warn {
    ($($arg:tt)*) => { tagged_event!(RequestWarn, warn, $($arg)*) }
}

#[macro_export]
macro_rules! request_info {
    ($($arg:tt)*) => { tagged_event!(RequestInfo, info, $($arg)*) }
}

#[macro_export]
macro_rules! request_trace {
    ($($arg:tt)*) => { tagged_event!(RequestTrace, trace, $($arg)*) }
}

#[macro_export]
macro_rules! security_critical {
    ($($arg:tt)*) => { tagged_event!(SecurityCritical, error, $($arg)*) }
}

#[macro_export]
macro_rules! security_info {
    ($($arg:tt)*) => { tagged_event!(SecurityInfo, info, $($arg)*) }
}

#[macro_export]
macro_rules! security_access {
    ($($arg:tt)*) => { tagged_event!(SecurityAccess, info, $($arg)*) }
}

#[macro_export]
macro_rules! filter_error {
    ($($arg:tt)*) => { tagged_event!(FilterError, error, $($arg)*) }
}

#[macro_export]
macro_rules! filter_warn {
    ($($arg:tt)*) => { tagged_event!(FilterWarn, warn, $($arg)*) }
}

#[macro_export]
macro_rules! filter_info {
    ($($arg:tt)*) => { tagged_event!(FilterInfo, info, $($arg)*) }
}

#[macro_export]
macro_rules! filter_trace {
    ($($arg:tt)*) => { tagged_event!(FilterTrace, trace, $($arg)*) }
}

#[macro_export]
macro_rules! perf_trace {
    ($($arg:tt)*) => { tagged_event!(PerfTrace, trace, $($arg)*) }
}
