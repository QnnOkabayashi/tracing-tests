use chrono::Utc;
use std::fmt;
use tracing::field::{Field, Visit};
use tracing::{Event, Level};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Registry;

pub enum LogFmt {
    Json,
    Pretty,
}

impl LogFmt {
    pub fn format_event<W: fmt::Write>(
        &self,
        writer: &mut W,
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

                event.record(&mut Visitor::new(
                    writer,
                    |w: &mut W, value: &dyn fmt::Debug| write!(w, r#""message":"{:?}""#, value),
                    |w: &mut W, field: &Field, value: &dyn fmt::Debug| {
                        write!(w, r#","{}":"{:?}""#, field.name(), value)
                    },
                ));

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
                        write!(writer, r#""{}""#, span.name())?;
                    }
                }
                writeln!(writer, "]}}")
            }
            Self::Pretty => {
                write!(
                    writer,
                    "{ts} {level:>7} ",
                    ts = ts.format("%b %m %H:%M:%S.%3f"),
                    level = match *event.metadata().level() {
                        Level::TRACE => "Trace 📍",
                        Level::DEBUG => "Debug 🐛",
                        Level::INFO => "Info 🔐",
                        Level::WARN => "Warn 🚧",
                        Level::ERROR => "Error 🚨",
                    },
                )?;

                if let Some(spans) = ctx.event_scope(event) {
                    for span in spans.from_root() {
                        write!(writer, "{}:", span.name())?;
                    }
                }

                write!(writer, " {}: ", event.metadata().target())?;

                event.record(&mut Visitor::new(
                    writer,
                    |w: &mut W, value: &dyn fmt::Debug| write!(w, "{:?}", value),
                    |w: &mut W, field: &Field, value: &dyn fmt::Debug| {
                        write!(w, " | {}={:?}", field.name(), value)
                    },
                ));

                writeln!(writer)
            }
        }
    }
}

struct Visitor<'writer, W, MsgFn, KeyFn> {
    first: bool,
    writer: &'writer mut W,
    on_msg: MsgFn,
    on_key: KeyFn,
}

impl<'writer, W, MsgFn, KeyFn> Visitor<'writer, W, MsgFn, KeyFn> {
    fn new(writer: &'writer mut W, on_msg: MsgFn, on_key: KeyFn) -> Self {
        Visitor {
            first: true,
            writer,
            on_msg,
            on_key,
        }
    }
}

impl<'writer, W, MsgFn, KeyFn> Visit for Visitor<'writer, W, MsgFn, KeyFn>
where
    W: fmt::Write,
    MsgFn: FnMut(&mut W, &dyn fmt::Debug) -> fmt::Result,
    KeyFn: FnMut(&mut W, &Field, &dyn fmt::Debug) -> fmt::Result,
{
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.first {
            assert!(field.name() == "message", "First field must be \"message\"");
            self.first = false;
            (self.on_msg)(self.writer, value)
        } else {
            (self.on_key)(self.writer, field, value)
        }
        .expect("Write failed")
    }
}
