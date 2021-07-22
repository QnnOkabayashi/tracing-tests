use crate::subscriber::{MyEvent, MyEventTag, MyProcessedLogs, MyProcessedSpan};
use serde::{ser::SerializeStruct, Serialize};
use serde_json;
use std::fmt;
use std::io::{self, Write as _};
use tracing::Level;
use tracing_serde::AsSerde;

#[derive(Clone, Copy, Debug)]
pub enum LogFmt {
    Json,
    Pretty,
}

const EVENT_UUID: &str = "00000000-0000-0000-0000-000000000000";

pub fn format_pretty(processed_logs: MyProcessedLogs) -> Vec<u8> {
    #[derive(Clone, Copy)]
    enum Fill {
        Void,
        Line,
        Fork,
        Turn,
    }

    impl fmt::Display for Fill {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            use Fill::*;
            f.write_str(match self {
                Void => "   ",
                Line => "│  ",
                Fork => "┝━ ",
                Turn => "┕━ ",
            })
        }
    }

    fn fmt_rec(
        tree: &MyProcessedLogs,
        indent: &mut Vec<Fill>,
        uuid: Option<&str>,
        root_duration: Option<f64>,
        writer: &mut Vec<u8>,
    ) -> io::Result<()> {
        use Fill::*;
        match tree {
            MyProcessedLogs::Event(event) => {
                use crate::subscriber::MyEventTag::*;

                const ERROR_EMOJI: &str = "🚨";
                const WARN_EMOJI: &str = "🚧";
                const INFO_EMOJI: &str = "💬";
                const DEBUG_EMOJI: &str = "🐛";
                const TRACE_EMOJI: &str = "📍";

                let uuid = uuid.unwrap_or(EVENT_UUID);

                let timestamp_fmt = event.timestamp.to_rfc3339();

                // level, emoji, tag

                let emoji = event
                    .tag
                    .as_ref()
                    .map(|tag| match tag {
                        AdminError | RequestError | FilterError => ERROR_EMOJI,
                        AdminWarn | RequestWarn | FilterWarn => WARN_EMOJI,
                        AdminInfo | RequestInfo | SecurityInfo | FilterInfo => INFO_EMOJI,
                        RequestTrace | FilterTrace | PerfTrace => TRACE_EMOJI,
                        SecurityCritical => "🔐",
                        SecurityAccess => "🔓",
                    })
                    .unwrap_or_else(|| match event.level {
                        Level::ERROR => ERROR_EMOJI,
                        Level::WARN => WARN_EMOJI,
                        Level::INFO => INFO_EMOJI,
                        Level::DEBUG => DEBUG_EMOJI,
                        Level::TRACE => TRACE_EMOJI,
                    });

                let tag_fmt = event
                    .tag
                    .as_ref()
                    .map(|tag| match tag {
                        AdminError => "admin.error",
                        AdminWarn => "admin.warn",
                        AdminInfo => "admin.info",
                        RequestError => "request.error",
                        RequestWarn => "request.warn",
                        RequestInfo => "request.info",
                        RequestTrace => "request.trace",
                        SecurityCritical => "security.critical",
                        SecurityInfo => "security.info",
                        SecurityAccess => "security.access",
                        FilterError => "filter.error",
                        FilterWarn => "filter.warn",
                        FilterInfo => "filter.info",
                        FilterTrace => "filter.trace",
                        PerfTrace => "perf.trace",
                    })
                    .unwrap_or_else(|| match event.level {
                        Level::ERROR => "_.error",
                        Level::WARN => "_.warn",
                        Level::INFO => "_.info",
                        Level::DEBUG => "_.debug",
                        Level::TRACE => "_.trace",
                    });

                write!(writer, "{} {} {:<8} ", uuid, timestamp_fmt, event.level)?;

                for fill in indent.iter() {
                    write!(writer, "{}", fill)?;
                }

                write!(writer, "{} [{}]: {}", emoji, tag_fmt, event.message)?;

                for (field, value) in event.values.iter() {
                    write!(writer, " | {}: {}", field, value)?;
                }

                writeln!(writer)
            }
            MyProcessedLogs::Span(span) => {
                let uuid = span
                    .uuid
                    .as_ref()
                    .map(String::as_str)
                    .or(uuid)
                    .expect("Span has no associated UUID, this is a bug");

                let timestamp_fmt = span.timestamp.to_rfc3339();

                let total_duration = span.total_duration as f64;

                let root_duration = root_duration.unwrap_or(total_duration);

                let total_load = 100.0 * total_duration / root_duration;

                struct DurationDisplay(f64);

                impl fmt::Display for DurationDisplay {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        let mut t = self.0;
                        for unit in ["ns", "µs", "ms", "s"].iter() {
                            if t < 10.0 {
                                return write!(f, "{:.2}{}", t, unit);
                            } else if t < 100.0 {
                                return write!(f, "{:.1}{}", t, unit);
                            } else if t < 1000.0 {
                                return write!(f, "{:.0}{}", t, unit);
                            }
                            t /= 1000.0;
                        }
                        write!(f, "{:.0}s", t * 1000.0)
                    }
                }

                write!(writer, "{} {} TRACE    ", uuid, timestamp_fmt)?;

                for fill in indent.iter() {
                    write!(writer, "{}", fill)?;
                }

                write!(
                    writer,
                    "{} [ {} | ",
                    span.name,
                    DurationDisplay(total_duration)
                )?;

                if span.nested_duration > 0 {
                    let direct_load =
                        100.0 * (total_duration - span.nested_duration as f64) / root_duration;
                    write!(writer, "{:.3}% / ", direct_load)?;
                }

                writeln!(writer, "{:.3}% ]", total_load)?;

                if let Some((last, remaining)) = span.processed_buf.split_last() {
                    // This span has children
                    // This is for what wraps the left of this span
                    match indent.last_mut() {
                        Some(f @ Turn) => *f = Void,
                        Some(f @ Fork) => *f = Line,
                        _ => {}
                    }

                    // Need to extend by one
                    indent.push(Fork);
                    for logs in remaining {
                        // Reset to Fork
                        indent.last_mut().map(|fill| *fill = Fork);
                        fmt_rec(logs, indent, Some(uuid), Some(root_duration), writer)?;
                    }

                    // Last child, set to Turn
                    indent.last_mut().map(|fill| *fill = Turn);
                    fmt_rec(last, indent, Some(uuid), Some(root_duration), writer)?;

                    indent.pop();
                } else {
                    // this span has no children
                }

                Ok(())
            }
        }
    }

    let mut writer = vec![];
    let mut indent = vec![];
    fmt_rec(&processed_logs, &mut indent, None, None, &mut writer).expect("Write failed");
    writer
}

pub fn format_json(processed_logs: MyProcessedLogs) -> Vec<u8> {
    fn fmt_rec<'a>(
        tree: &MyProcessedLogs,
        spans: &'a mut Vec<&'static str>,
        uuid: Option<&'a str>,
        mut writer: &mut Vec<u8>,
    ) -> io::Result<()> {
        match tree {
            MyProcessedLogs::Event(event) => {
                struct SerializeEvent<'a> {
                    event: &'a MyEvent,
                    uuid: &'a str,
                    spans: &'a mut Vec<&'static str>,
                }

                impl<'a> Serialize for SerializeEvent<'a> {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: serde::Serializer,
                    {
                        let mut model = serializer.serialize_struct("event", 7)?;
                        model.serialize_field("uuid", self.uuid)?;
                        model.serialize_field("timestamp", &self.event.timestamp.to_rfc3339())?;
                        model.serialize_field("level", &self.event.level.as_serde())?;
                        model.serialize_field("message", &self.event.message)?;
                        model.serialize_field("log-type", "event")?;
                        model.serialize_field("tag", &self.event.tag.map(MyEventTag::pretty))?;
                        model.serialize_field("spans", self.spans)?;
                        model.end()
                    }
                }

                let serialize_event = SerializeEvent {
                    event,
                    uuid: uuid.unwrap_or(EVENT_UUID),
                    spans,
                };

                serde_json::to_writer(&mut writer, &serialize_event).map_err(io::Error::from)?;
                writeln!(writer)
            }
            MyProcessedLogs::Span(span) => {
                struct SerializeSpan<'a> {
                    span: &'a MyProcessedSpan,
                    uuid: &'a str,
                }

                impl<'a> Serialize for SerializeSpan<'a> {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: serde::Serializer,
                    {
                        let mut model = serializer.serialize_struct("event", 7)?;
                        model.serialize_field("uuid", self.uuid)?;
                        model.serialize_field("timestamp", &self.span.timestamp.to_rfc3339())?;
                        model.serialize_field("level", "TRACE")?;
                        model.serialize_field("message", &self.span.name)?;
                        model.serialize_field("log-type", "span")?;
                        model.serialize_field("nanos-nested", &self.span.nested_duration)?;
                        model.serialize_field("nanos-total", &self.span.total_duration)?;
                        model.end()
                    }
                }

                let uuid = span
                    .uuid
                    .as_ref()
                    .map(String::as_str)
                    .or(uuid)
                    .expect("Span has no associated UUID, this is a bug");

                let serialize_span = SerializeSpan { span, uuid };

                serde_json::to_writer(&mut writer, &serialize_span).map_err(io::Error::from)?;
                writeln!(writer)?;

                // format stuff in child spans
                spans.push(span.name);
                for logs in span.processed_buf.iter() {
                    fmt_rec(logs, spans, Some(uuid), writer)?;
                }
                spans.pop();
                Ok(())
            }
        }
    }

    let mut writer = vec![];
    let mut spans = vec![];
    fmt_rec(&processed_logs, &mut spans, None, &mut writer).expect("Write failed");
    writer
}
