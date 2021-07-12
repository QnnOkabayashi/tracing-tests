use chrono::Utc;
use std::fmt;
use tracing::field::{Field, Visit};
use tracing::Event;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Registry;
// use tracing_subscriber::fmt::Layer;

pub enum LogFmt {
    Json,
    Pretty,
}

impl LogFmt {
    pub fn format_event(
        &self,
        writer: &mut impl fmt::Write,
        event: &Event,
        ctx: &Context<Registry>,
    ) -> fmt::Result {
        let ts = Utc::now();
        match *self {
            Self::Json => {
                write!(
                    writer,
                    r#"{{"timestamp":"{ts}","level":"{level}","fields":{{"#,
                    ts = ts.format("%b %m %H:%M:%S.%3f"),
                    level = event.metadata().level(),
                )?;

                struct Visitor<'writer, W> {
                    first: bool,
                    writer: &'writer mut W,
                }

                impl<'writer, W: fmt::Write> Visit for Visitor<'writer, W> {
                    fn record_bool(&mut self, _: &Field, _: bool) {}

                    fn record_error(&mut self, _: &Field, _: &(dyn std::error::Error + 'static)) {}

                    fn record_i64(&mut self, _: &Field, _: i64) {}

                    fn record_u64(&mut self, _: &Field, _: u64) {}

                    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                        if self.first {
                            self.first = false;
                        } else {
                            self.writer.write_char(',').expect("Write failed");
                        }
                        write!(self.writer, r#""{}":"{:?}""#, field.name(), value)
                            .expect("Write failed");
                    }
                }

                event.record(&mut Visitor {
                    first: true,
                    writer,
                });

                write!(
                    writer,
                    r#"}},"target":"{target}","spans":["#,
                    target = event.metadata().target(),
                )?;

                if let Some(scope) = ctx.event_scope(event) {
                    let mut first = true;
                    for span in scope {
                        if first {
                            first = false;
                        } else {
                            writer.write_char(',')?;
                        }
                        write!(writer, r#"{{"name":"{}"}}"#, span.name())?;
                    }
                }
                writeln!(writer, "]}}")
            }
            Self::Pretty => {
                write!(
                    writer,
                    "{ts} {level:>5} ",
                    ts = ts.format("%b %m %H:%M:%S.%3f"),
                    level = event.metadata().level(),
                )?;

                if let Some(spans) = ctx.event_scope(event) {
                    for span in spans.from_root() {
                        write!(writer, "{}:", span.name())?;
                    }
                }

                write!(writer, " {}: ", event.metadata().target())?;

                struct Visitor<'writer, W> {
                    writer: &'writer mut W,
                }

                impl<'writer, W: fmt::Write> Visit for Visitor<'writer, W> {
                    fn record_bool(&mut self, _: &Field, _: bool) {}

                    fn record_error(&mut self, _: &Field, _: &(dyn std::error::Error + 'static)) {}

                    fn record_i64(&mut self, _: &Field, _: i64) {}

                    fn record_u64(&mut self, _: &Field, _: u64) {}

                    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                        if field.name() == "message" {
                            write!(self.writer, "{:?}", value).expect("Write failed")
                        } else {
                            write!(self.writer, " | {}={:?}", field.name(), value)
                                .expect("Write failed")
                        }
                    }
                }

                event.record(&mut Visitor { writer });

                writeln!(writer)
            }
        }
    }
}
