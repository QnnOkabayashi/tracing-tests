use crate::subscriber::MyProcessedLogs;
use tracing::Level;

use std::fmt::{self, Write as _};

#[derive(Clone, Copy, Debug)]
pub enum LogFmt {
    Json,
    Pretty,
}

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

pub fn format_pretty(processed_logs: MyProcessedLogs) -> String {
    fn fmt_rec(
        tree: &MyProcessedLogs,
        indent: &mut Vec<Fill>,
        uuid: Option<&str>,
        writer: &mut String,
    ) -> fmt::Result {
        use Fill::*;
        match tree {
            MyProcessedLogs::Event(event) => {
                use crate::subscriber::MyEventTag::*;

                const ERROR_EMOJI: &str = "🚨";
                const WARN_EMOJI: &str = "🚧";
                const INFO_EMOJI: &str = "💬";
                const DEBUG_EMOJI: &str = "🐛";
                const TRACE_EMOJI: &str = "📍";

                let uuid = uuid.unwrap_or("00000000-0000-0000-0000-000000000000");

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
                    DurationDisplay(span.duration)
                )?;

                if let Some(direct_load) = span.direct_load {
                    write!(writer, "{:.3}% / ", direct_load)?;
                }

                writeln!(writer, "{:.3}% ]", span.total_load)?;

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
                    for event in remaining {
                        // Reset to Fork
                        indent.last_mut().map(|fill| *fill = Fork);
                        fmt_rec(event, indent, Some(uuid), writer)?;
                    }

                    // Last child, set to Turn
                    indent.last_mut().map(|fill| *fill = Turn);
                    fmt_rec(last, indent, Some(uuid), writer)?;

                    indent.pop();
                } else {
                    // this span has no children
                }

                Ok(())
            }
        }
    }

    let mut writer = String::new();
    let mut indent = vec![];
    fmt_rec(&processed_logs, &mut indent, None, &mut writer).expect("Write failed");
    writer
}
