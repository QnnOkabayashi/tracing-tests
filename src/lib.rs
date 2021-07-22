pub mod formatter;
pub mod subscriber;
pub(crate) mod timings;

#[macro_use]
pub mod macros;

#[cfg(test)]
mod tests {
    use crate::formatter::LogFmt;
    use crate::subscriber::{MyLogs, MySubscriber};
    use std::io;
    use tokio;
    use tokio::sync::mpsc::unbounded_channel as unbounded;
    use tokio::time::{sleep, Duration};
    use tracing::{self, debug, info, instrument, trace, trace_span};
    use uuid::Uuid;

    #[tokio::test]
    async fn async_tests() {
        let (log_tx, mut log_rx) = unbounded::<(LogFmt, MyLogs)>();

        let subscriber = MySubscriber::new(LogFmt::Json, log_tx);
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

        while let Some((fmt, logs)) = log_rx.recv().await {
            let processed_logs = logs.process();
            let formatted_logs = match fmt {
                LogFmt::Json => crate::formatter::format_json(processed_logs),
                LogFmt::Pretty => crate::formatter::format_pretty(processed_logs),
            };
            io::Write::write(&mut io::stderr(), &formatted_logs[..]).expect("Write failed");
        }
    }

    #[tokio::test]
    async fn deep_spans() {
        let (log_tx, mut log_rx) = unbounded::<(LogFmt, MyLogs)>();

        let subscriber = MySubscriber::new(LogFmt::Json, log_tx);
        let guard = tracing::subscriber::set_default(subscriber);

        trace_span!("try_from_entry_ro").in_scope(|| {
            trace_span!("server::internal_search").in_scope(|| {
                filter_info!("Some filter info...");
                trace_span!("server::search").in_scope(|| {
                    trace_span!("be::search").in_scope(|| {
                        trace_span!("be::search -> filter2idl").in_scope(|| {
                            trace_span!("be::idl_arc_sqlite::get_idl")
                                .in_scope(|| filter_info!("Some filter info..."));
                            trace_span!("be::idl_arc_sqlite::get_idl").in_scope(|| {
                                admin_error!("Oopsies, an admin error occurred :)");
                                debug!("An untagged debug log")
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

        while let Some((fmt, logs)) = log_rx.recv().await {
            let processed_logs = logs.process();
            let formatted_logs = match fmt {
                LogFmt::Json => crate::formatter::format_json(processed_logs),
                LogFmt::Pretty => crate::formatter::format_pretty(processed_logs),
            };
            io::Write::write(&mut io::stderr(), &formatted_logs[..]).expect("Write failed");
        }

        println!("done");
    }
}
