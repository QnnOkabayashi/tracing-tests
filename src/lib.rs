pub mod formatter;
pub mod subscriber;
pub(crate) mod timings;

#[macro_use]
pub mod macros;

#[cfg(test)]
mod tests {
    use crate::subscriber::MySubscriber;
    use tracing;

    #[test]
    fn trace_test() {
        tracing::subscriber::with_default(MySubscriber::new(), || {
            tracing::trace_span!("A").in_scope(|| {
                tracing::trace_span!("B").in_scope(|| {
                    // non-string values are ignored
                    tracing::error!(message = "oh no!", tag = "admin", age = 10);
                    alarm!("oh man, it's {}", 3);
                })
            })
        });
    }

    #[test]
    fn tracing_subscriber_builtins() {
        use tracing::Level;
        use tracing_subscriber::fmt::format::FmtSpan;

        let subscriber = tracing_subscriber::fmt()
            // SETTINGS
            .with_max_level(Level::TRACE)
            // .with_ansi(false) // no colors (good for .log files)
            // JSON
            // .json() // machine-readable
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
            tracing::trace_span!("A").in_scope(|| {
                tracing::trace_span!("B").in_scope(|| {
                    tracing::trace_span!("C").in_scope(|| {
                        tracing::error!("oh no!");
                    })
                })
            })
        })
    }
}
