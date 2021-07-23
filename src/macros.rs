#[macro_export]
macro_rules! alarm {
    ($($arg:tt)*) => {{
        use tracing;
        tracing::error!(alarm = true, $($arg)*);
    }};
}

#[allow(unused_macros)]
macro_rules! tagged_event {
    ($level:ident, $logtag:path, $($arg:tt)*) => {{
        use tracing;
        fn assert_eventtagset<T: crate::subscriber::EventTagSet>(_x: &T) {}
        assert_eventtagset(&$logtag);
        let event_tag: u64 = $logtag.into();
        tracing::event!(tracing::Level::$level, event_tag, $($arg)*)
    }}
}
