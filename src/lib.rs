pub mod formatter;
pub mod subscriber;
pub(crate) mod timings;

#[macro_use]
pub mod macros;

#[cfg(test)]
mod tests {
    use crate::subscriber::MySubscriber;
    use tokio;
    use tokio::time::{sleep, Duration};
    use tracing::{self, info, info_span, Level};
    use tracing_subscriber::fmt::format::FmtSpan;

    #[test]
    fn trace_test() {
        tracing::subscriber::with_default(MySubscriber::pretty(), || {
            tracing::trace_span!("A").in_scope(|| {
                tracing::trace_span!("B").in_scope(|| {
                    tracing::error!(tag = "admin", "oh no!");
                    alarm!("oh man, it's {}", 3);
                })
            })
        });
    }

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
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            // .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE) // logs when `span`s are initialized or terminated
            .finish();

        let subscriber = MySubscriber::pretty();

        tracing::subscriber::set_global_default(subscriber).unwrap();

        // #[tracing::instrument]
        async fn first() {
            let span = info_span!("first");

            span.in_scope(|| {
                info!("before");
            });

            sleep(Duration::from_millis(500)).await;

            span.in_scope(|| {
                info!("after");
            });
        }

        // #[tracing::instrument]
        async fn second() {
            let span = info_span!("second");

            sleep(Duration::from_millis(250)).await;

            span.in_scope(|| {
                info!("Going to sleep...");
            });

            sleep(Duration::from_millis(500)).await;

            span.in_scope(|| {
                info!("Awake!");
            });
        }

        let a = first();
        let b = second();

        tokio::join!(a, b);
    }
}
