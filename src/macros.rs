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

macro_rules! tagged_event {
    ($logtag:ident, $event:ident, $($arg:tt)*) => {{
        use tracing;
        tracing::$event!(tag = tracing::field::display(stringify!($logtag)), $($arg)*)
    }}
}

#[macro_export]
macro_rules! admin_error {
    ($($arg:tt)*) => { tagged_event!(admin, error, $($arg)*) }
}

#[macro_export]
macro_rules! admin_warn {
    ($($arg:tt)*) => { tagged_event!(admin, warn, $($arg)*) }
}

#[macro_export]
macro_rules! admin_info {
    ($($arg:tt)*) => { tagged_event!(admin, info, $($arg)*) }
}

#[macro_export]
macro_rules! request_error {
    ($($arg:tt)*) => { tagged_event!(request, error, $($arg)*) }
}

#[macro_export]
macro_rules! request_warn {
    ($($arg:tt)*) => { tagged_event!(admin, warn, $($arg)*) }
}

#[macro_export]
macro_rules! request_info {
    ($($arg:tt)*) => { tagged_event!(request, info, $($arg)*) }
}

#[macro_export]
macro_rules! request_trace {
    ($($arg:tt)*) => { tagged_event!(request, trace, $($arg)*) }
}

#[macro_export]
macro_rules! security_critical {
    ($($arg:tt)*) => { tagged_event!(security, error, $($arg)*) }
}

#[macro_export]
macro_rules! security_info {
    ($($arg:tt)*) => { tagged_event!(security, info, $($arg)*) }
}

#[macro_export]
macro_rules! security_access {
    ($($arg:tt)*) => { tagged_event!(security, trace, $($arg)*) }
}

#[macro_export]
macro_rules! filter_error {
    ($($arg:tt)*) => { tagged_event!(filter, error, $($arg)*) }
}

#[macro_export]
macro_rules! filter_warn {
    ($($arg:tt)*) => { tagged_event!(filter, warn, $($arg)*) }
}

#[macro_export]
macro_rules! filter_info {
    ($($arg:tt)*) => { tagged_event!(filter, info, $($arg)*) }
}

#[macro_export]
macro_rules! filter_trace {
    ($($arg:tt)*) => { tagged_event!(filter, trace, $($arg)*) }
}

#[macro_export]
macro_rules! perf_trace {
    ($($arg:tt)*) => { tagged_event!(filter, trace, $($arg)*) }
}
