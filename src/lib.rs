pub mod formatter;
pub mod subscriber;
mod timings;

#[macro_use]
pub mod kanidm;

#[macro_use]
pub mod macros;

mod middleware;

#[cfg(test)]
mod tests {
    use crate::kanidm::KanidmEventTag;
    use crate::middleware::TreeMiddleware;
    use crate::subscriber::{TreeProcessor, TreeSubscriber};
    use tokio;
    use tokio::sync::mpsc::unbounded_channel as unbounded;
    use tokio::time::{sleep, Duration};
    use tracing::{self, debug, info, instrument, trace, trace_span};
    use uuid::Uuid;

    #[tokio::test]
    async fn async_tests() {
        let (log_tx, mut log_rx) = unbounded::<TreeProcessor<KanidmEventTag>>();

        let subscriber = TreeSubscriber::pretty(log_tx);
        let guard = tracing::subscriber::set_default(subscriber);

        #[instrument]
        async fn first(uuid: Uuid) {
            filter_error!("First event");
            sleep(Duration::from_millis(500)).await;
            admin_error!("Third event");
        }

        #[instrument]
        async fn second() {
            sleep(Duration::from_millis(250)).await;
            admin_error!("Second event");
            sleep(Duration::from_millis(500)).await;
            filter_error!("Fourth event");
        }

        let uuid = Uuid::new_v4();

        info!("Going to use this UUID: {}", uuid);

        let a = first(uuid);
        let b = second();

        tokio::join!(a, b);

        // drop so all the senders are gone
        drop(guard);

        while let Some(processor) = log_rx.recv().await {
            processor.process().expect("Write failed");
        }
    }

    #[tokio::test]
    async fn deep_spans() {
        let (log_tx, mut log_rx) = unbounded::<TreeProcessor<KanidmEventTag>>();

        let subscriber = TreeSubscriber::pretty(log_tx);
        let guard = tracing::subscriber::set_default(subscriber);

        trace_span!("try_from_entry_ro", output = "test-out/deep_spans_test.log").in_scope(|| {
            trace_span!("server::internal_search").in_scope(|| {
                filter_info!("Some filter info...");
                trace_span!("server::search").in_scope(|| {
                    trace_span!("be::search").in_scope(|| {
                        trace_span!("be::search -> filter2idl").in_scope(|| {
                            trace_span!("be::idl_arc_sqlite::get_idl")
                                .in_scope(|| filter_info!("Some filter info..."));
                            trace_span!("be::idl_arc_sqlite::get_idl").in_scope(|| {
                                admin_error!("Oopsies, an admin error occurred :)");
                                debug!("An untagged debug log");
                                alarm!(
                                    alive = false,
                                    status = "very sad",
                                    "there's been a big mistake"
                                )
                            })
                        });
                        trace_span!("be::idl_arc_sqlite::get_identry").in_scope(|| {
                            security_critical!("A security critical log");
                            security_access!("A security access log")
                        })
                    });
                    trace_span!("server::search<filter_resolve>")
                        .in_scope(|| filter_warn!("Some filter warning lol"))
                })
            });
            trace!("We finished!")
        });

        // drop so all the senders are gone
        drop(guard);

        while let Some(processor) = log_rx.recv().await {
            processor.process().expect("Write failed");
        }

        println!("done");
    }

    #[tokio::test]
    async fn middleware_test() {
        let (log_tx, mut log_rx) = unbounded::<TreeProcessor<KanidmEventTag>>();

        let subscriber = TreeSubscriber::pretty(log_tx);
        tracing::subscriber::set_global_default(subscriber).unwrap();

        tokio::spawn(async move {
            while let Some(processor) = log_rx.recv().await {
                processor.process().expect("Write failed");
            }
        });

        let mut app = tide::new();
        app.with(TreeMiddleware::new("test_out/middleware_test.log"));
        app.at("/").get(|_| async { Ok("Hello, world!") });
        app.at("/sara").get(|_| async { Ok("omg it's sara!") });
        app.listen("127.0.0.1:8080").await.unwrap();
    }
}
