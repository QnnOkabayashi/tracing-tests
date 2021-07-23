use crate::subscriber::EventTagSet;
use std::convert::TryFrom;

#[derive(Debug, Clone, Copy)]
pub enum KanidmEventTag {
    AdminError,
    AdminWarn,
    AdminInfo,
    RequestError,
    RequestWarn,
    RequestInfo,
    RequestTrace,
    SecurityCritical,
    SecurityInfo,
    SecurityAccess,
    FilterError,
    FilterWarn,
    FilterInfo,
    FilterTrace,
    PerfTrace,
}

impl EventTagSet for KanidmEventTag {
    fn pretty(self) -> &'static str {
        match self {
            KanidmEventTag::AdminError => "admin.error",
            KanidmEventTag::AdminWarn => "admin.warn",
            KanidmEventTag::AdminInfo => "admin.info",
            KanidmEventTag::RequestError => "request.error",
            KanidmEventTag::RequestWarn => "request.error",
            KanidmEventTag::RequestInfo => "request.info",
            KanidmEventTag::RequestTrace => "request.trace",
            KanidmEventTag::SecurityCritical => "security.critical",
            KanidmEventTag::SecurityInfo => "security.info",
            KanidmEventTag::SecurityAccess => "security.access",
            KanidmEventTag::FilterError => "filter.error",
            KanidmEventTag::FilterWarn => "filter.warn",
            KanidmEventTag::FilterInfo => "filter.info",
            KanidmEventTag::FilterTrace => "filter.trace",
            KanidmEventTag::PerfTrace => "perf.trace",
        }
    }

    fn emoji(self) -> &'static str {
        use KanidmEventTag::*;
        match self {
            AdminError | RequestError | FilterError => "🚨",
            AdminWarn | RequestWarn | FilterWarn => "🚧",
            AdminInfo | RequestInfo | SecurityInfo | FilterInfo => "💬",
            RequestTrace | FilterTrace | PerfTrace => "📍",
            SecurityCritical => "🔐",
            SecurityAccess => "🔓",
        }
    }
}

impl From<KanidmEventTag> for u64 {
    fn from(tag: KanidmEventTag) -> Self {
        use KanidmEventTag::*;
        match tag {
            AdminError => 0,
            AdminWarn => 1,
            AdminInfo => 2,
            RequestError => 3,
            RequestWarn => 4,
            RequestInfo => 5,
            RequestTrace => 6,
            SecurityCritical => 7,
            SecurityInfo => 8,
            SecurityAccess => 9,
            FilterError => 10,
            FilterWarn => 11,
            FilterInfo => 12,
            FilterTrace => 13,
            PerfTrace => 14,
        }
    }
}

impl TryFrom<u64> for KanidmEventTag {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        use KanidmEventTag::*;
        match value {
            0 => Ok(AdminError),
            1 => Ok(AdminWarn),
            2 => Ok(AdminInfo),
            3 => Ok(RequestError),
            4 => Ok(RequestWarn),
            5 => Ok(RequestInfo),
            6 => Ok(RequestTrace),
            7 => Ok(SecurityCritical),
            8 => Ok(SecurityInfo),
            9 => Ok(SecurityAccess),
            10 => Ok(FilterError),
            11 => Ok(FilterWarn),
            12 => Ok(FilterInfo),
            13 => Ok(FilterTrace),
            14 => Ok(PerfTrace),
            _ => Err(()),
        }
    }
}

#[macro_export]
macro_rules! admin_error {
    ($($arg:tt)*) => { tagged_event!(ERROR, KanidmEventTag::AdminError, $($arg)*) }
}

#[macro_export]
macro_rules! admin_warn {
    ($($arg:tt)*) => { tagged_event!(WARN, KanidmEventTag::AdminWarn, $($arg)*) }
}

#[macro_export]
macro_rules! admin_info {
    ($($arg:tt)*) => { tagged_event!(INFO, KanidmEventTag::AdminInfo, $($arg)*) }
}

#[macro_export]
macro_rules! request_error {
    ($($arg:tt)*) => { tagged_event!(ERROR, KanidmEventTag::RequestError, $($arg)*) }
}

#[macro_export]
macro_rules! request_warn {
    ($($arg:tt)*) => { tagged_event!(WARN, KanidmEventTag::RequestWarn, $($arg)*) }
}

#[macro_export]
macro_rules! request_info {
    ($($arg:tt)*) => { tagged_event!(INFO, KanidmEventTag::RequestInfo, $($arg)*) }
}

#[macro_export]
macro_rules! request_trace {
    ($($arg:tt)*) => { tagged_event!(TRACE, KanidmEventTag::RequestTrace, $($arg)*) }
}

#[macro_export]
macro_rules! security_critical {
    ($($arg:tt)*) => { tagged_event!(INFO, KanidmEventTag::SecurityCritical, $($arg)*) }
}

#[macro_export]
macro_rules! security_info {
    ($($arg:tt)*) => { tagged_event!(INFO, KanidmEventTag::SecurityInfo, $($arg)*) }
}

#[macro_export]
macro_rules! security_access {
    ($($arg:tt)*) => { tagged_event!(INFO, KanidmEventTag::SecurityAccess, $($arg)*) }
}

#[macro_export]
macro_rules! filter_error {
    ($($arg:tt)*) => { tagged_event!(ERROR, KanidmEventTag::FilterError, $($arg)*) }
}

#[macro_export]
macro_rules! filter_warn {
    ($($arg:tt)*) => { tagged_event!(WARN, KanidmEventTag::FilterWarn, $($arg)*) }
}

#[macro_export]
macro_rules! filter_info {
    ($($arg:tt)*) => { tagged_event!(INFO, KanidmEventTag::FilterInfo, $($arg)*) }
}

#[macro_export]
macro_rules! filter_trace {
    ($($arg:tt)*) => { tagged_event!(TRACE, KanidmEventTag::FilterTrace, $($arg)*) }
}

#[macro_export]
macro_rules! perf_trace {
    ($($arg:tt)*) => { tagged_event!(TRACE, KanidmEventTag::PerfTrace, $($arg)*) }
}
