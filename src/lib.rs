pub mod formatter;
pub mod subscriber;
pub(crate) mod timings;

#[macro_use]
pub mod macros;

#[cfg(test)]
mod tests {
    use crate::formatter::LogFmt;
    use crate::subscriber::{MyEvent, MySubscriber};
    use tokio;
    use tokio::sync::mpsc::unbounded_channel as unbounded;
    use tokio::time::{sleep, Duration};
    use tracing::{self, info, instrument, Level};
    use tracing_subscriber::fmt::format::FmtSpan;

    #[test]
    fn tracing_subscriber_builtins() {
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

    #[tokio::test]
    async fn async_tests() {
        let (log_tx, mut log_rx) = unbounded::<(LogFmt, Vec<MyEvent>)>();

        tokio::spawn(async move {
            while let Some((fmt, events)) = log_rx.recv().await {
                eprintln!("{}", fmt.format_events(events).unwrap());
            }
        });

        {
            let subscriber = MySubscriber::new(LogFmt::Pretty, log_tx);
            let _guard = tracing::subscriber::set_default(subscriber);

            #[instrument(level = "trace")]
            async fn first() {
                filter_error!("First event");
                sleep(Duration::from_millis(500)).await;
                admin_error!("Third event");
            }

            #[instrument(level = "trace", fields(timed = true))]
            async fn second() {
                sleep(Duration::from_millis(250)).await;
                admin_error!("Second event");
                sleep(Duration::from_millis(500)).await;
                filter_error!("Fourth event");
            }

            let a = first();
            let b = second();

            tokio::join!(a, b);
        }

        println!("done");
    }
}
