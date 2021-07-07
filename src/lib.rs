pub(crate) mod event;
pub mod subscriber;
pub(crate) mod timings;

#[cfg(test)]
mod tests {
    use crate::subscriber::MySubscriber;
    use tracing;

    #[test]
    fn trace_test() {
        tracing::subscriber::with_default(MySubscriber::new(), || {
            tracing::trace_span!("Wrapper").in_scope(|| {
                tracing::error!("oh no!");
            })
        })
    }

    #[test]
    fn tracing_subscriber_builtins() {
        use tracing::Level;
        use tracing_subscriber::field::MakeExt;
        use tracing_subscriber::fmt::format::{debug_fn, FmtSpan};

        let subscriber = tracing_subscriber::fmt()
            // SETTINGS
            .with_max_level(Level::TRACE)
            // .with_ansi(false) // no colors (good for .log files)
            // JSON
            .json() // machine-readable
            // .with_span_list(false) // displays all open spans
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE) // logs when `span`s are initialized or terminated
            // .fmt_fields({
            //     debug_fn(|writer, key, value| {
            //         match key.to_string().as_str() {
            //             "message" => write!(writer, "{:?}", value),
            //             _ => write!(writer, "{}={:?}", key, value),
            //         }
            //     }).delimited(" | ")
            // })
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::trace_span!("Wrapper").in_scope(|| {
                tracing::error!("oh no!");
            })
        })
    }
}
