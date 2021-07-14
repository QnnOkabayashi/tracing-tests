use crate::subscriber::MyEvent;
use std::fmt::{self, Write as _};
use tracing::Level;

#[derive(Clone, Copy, Debug)]
pub enum LogFmt {
    Json,
    Pretty,
}

impl LogFmt {
    pub fn format_events(&self, events: Vec<MyEvent>) -> Result<String, fmt::Error> {
        let mut writer = String::new();
        let cap;
        match *self {
            LogFmt::Json => {
                const MIN_LENGTH: usize = 37 + 25 + 5 + 12 + 23 + 4;
                let event_size = events.iter().map(|e| e.format_size(6, 3)).sum::<usize>();
                cap = (events.len() * MIN_LENGTH) + event_size;
                writer.reserve(cap);
                for e in events {
                    write!(
                        writer,
                        r#"{{"timestamp":"{}","level":"{}","fields":{{"#, // 37
                        e.timestamp.to_rfc3339(),                         // 25
                        e.level,                                          // 5
                    )?;

                    format_fields(
                        &mut writer,
                        e.fields,
                        |writer, value| write!(writer, r#""message":"{}""#, value), // 12
                        |writer, field, value| write!(writer, r#","{}":"{}""#, field, value), // value_pad: 6
                    )?;

                    write!(writer, r#"}},"target":"{}","spans":["#, e.target,)?; // 23

                    if let Some(scope) = e.spans {
                        let mut first = true;
                        for span in scope {
                            if first {
                                first = false;
                            } else {
                                write!(writer, ",")?; // 1, we can always count this even for first because it's so small
                            }
                            write!(writer, r#""{}""#, span)?; // 2
                        }
                        // span_pad: 3
                    }

                    writeln!(writer, "]}}")? // 4
                }
            }
            LogFmt::Pretty => {
                const MIN_LENGTH: usize = 19 + 10 + 3 + 1;
                let event_size = events.iter().map(|e| e.format_size(4, 1)).sum::<usize>();
                cap = (events.len() * MIN_LENGTH) + event_size;
                writer.reserve(cap);
                for e in events {
                    write!(
                        writer,
                        "{} {:>7} ",
                        e.timestamp.format("%b %m %H:%M:%S.%3f"), // 19
                        match e.level {
                            Level::TRACE => "Trace 📍",
                            Level::DEBUG => "Debug 🐛",
                            Level::INFO => "Info 🔐",
                            Level::WARN => "Warn 🚧",
                            Level::ERROR => "Error 🚨",
                        }, // 10, because emojis are 4 bytes
                    )?;

                    if let Some(scope) = e.spans {
                        for span in scope {
                            write!(writer, "{}:", span)?; // span_pad: 1
                        }
                    }

                    write!(writer, " {}: ", e.target)?; // 3

                    format_fields(
                        &mut writer,
                        e.fields,
                        |writer, value| write!(writer, "{}", value), // 0
                        |writer, field, value| write!(writer, " | {}={}", field, value), // value_pad: 4
                    )?;

                    writeln!(writer)?; // 1
                }
            }
        }
        Ok(writer)
    }
}

fn format_fields<MsgFn, KeyFn>(
    writer: &mut String,
    fields: Vec<(&'static str, String)>,
    mut msg_fn: MsgFn,
    mut key_fn: KeyFn,
) -> fmt::Result
where
    MsgFn: FnMut(&mut String, String) -> fmt::Result,
    KeyFn: FnMut(&mut String, &'static str, String) -> fmt::Result,
{
    let mut first = true;
    fields.into_iter().try_for_each(|(field, value)| {
        if first {
            assert!(field == "message", "First field must be \"message\"");
            first = false;
            msg_fn(writer, value)
        } else {
            key_fn(writer, field, value)
        }
    })
}
