use chrono::Utc;
use std::fmt;
use tracing::field::{Field, Visit};
use tracing::{Event, Id};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Registry;

pub enum LogFmt {
    Json,
    Pretty,
}

impl LogFmt {
    pub fn write_event(
        &self,
        writer: &mut dyn fmt::Write,
        event: &Event,
        ctx: &Context<Registry>,
    ) -> fmt::Result {
        match *self {
            Self::Json => {
                let ts = Utc::now();
                write!(
                    writer,
                    r#"{{"timestamp":"{ts}","level":"{level}","fields":{{"#,
                    ts = ts.format("%b %m %H:%M:%S.%3f"),
                    level = event.metadata().level(),
                )?;

                struct Visitor<'writer> {
                    first: bool,
                    writer: &'writer mut dyn fmt::Write,
                }

                impl<'writer> Visit for Visitor<'writer> {
                    fn record_str(&mut self, field: &Field, value: &str) {
                        if self.first {
                            self.first = false;
                        } else {
                            self.writer.write_char(',').expect("Write failed");
                        }
                        write!(self.writer, r#""{}":"{}""#, field.name(), value)
                            .expect("Write failed");
                    }

                    fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {}
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
                let ts = Utc::now();
                write!(
                    writer,
                    "{ts} {level} ",
                    ts = ts.format("%b %m %H:%M:%S.%3f"),
                    level = event.metadata().level(),
                )?;

                if let Some(spans) = ctx.event_scope(event) {
                    for span in spans.from_root() {
                        write!(writer, "{}:", span.name())?;
                    }
                }

                write!(writer, " {}: ", event.metadata().target())?;

                // NOTE: The first field MUST be "message"
                struct Visitor<'writer> {
                    first: bool,
                    writer: &'writer mut dyn fmt::Write,
                }

                impl<'writer> Visit for Visitor<'writer> {
                    fn record_str(&mut self, field: &Field, value: &str) {
                        if self.first {
                            self.first = false;
                            self.writer.write_str(value)
                        } else {
                            write!(self.writer, " | {}={}", field.name(), value)
                        }
                        .expect("Write failed");
                    }

                    fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {}
                }

                event.record(&mut Visitor {
                    first: true,
                    writer,
                });

                writeln!(writer)
            }
        }
    }

    // TODO
    pub fn new_span(
        &self,
        writer: &mut dyn fmt::Write,
        id: &Id,
        ctx: &Context<Registry>,
    ) -> fmt::Result {
        let _ = (writer, id, ctx);
        match *self {
            Self::Json => {
                todo!()
            }
            Self::Pretty => {
                todo!()
            }
        }
    }

    // TODO
    pub fn close_span(
        &self,
        writer: &mut dyn fmt::Write,
        id: &Id,
        ctx: &Context<Registry>,
    ) -> fmt::Result {
        let _ = (writer, id, ctx);
        match *self {
            Self::Json => {
                todo!()
            }
            Self::Pretty => {
                todo!()
            }
        }
    }
}
