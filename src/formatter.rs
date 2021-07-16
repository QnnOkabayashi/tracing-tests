use crate::subscriber::MyProcessedEventOrSpan;
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

use Fill::*;

pub fn format_pretty(processed_logs: MyProcessedEventOrSpan) -> String {
    fn fmt_rec(
        tree: &MyProcessedEventOrSpan,
        indent: &mut Vec<Fill>,
        writer: &mut String,
    ) -> fmt::Result {
        match tree {
            MyProcessedEventOrSpan::Event(event) => {
                use crate::subscriber::MyEventTag::*;

                const ERROR: &str = "🚨 ERROR";
                const WARN: &str = "🚧 WARN";
                const INFO: &str = "💬 INFO";
                const DEBUG: &str = "🐛 DEBUG";
                const TRACE: &str = "📌 TRACE";

                let timestamp_fmt = event.timestamp.format("%b %m %H:%M:%S.%3f");

                let level_fmt = event
                    .tag
                    .as_ref()
                    .map(|tag| match tag {
                        AdminError | RequestError | FilterError => ERROR,
                        AdminWarn | RequestWarn | FilterWarn => WARN,
                        AdminInfo | RequestInfo | SecurityInfo | FilterInfo => INFO,
                        RequestTrace | FilterTrace | PerfTrace => TRACE,
                        SecurityCritical => "🔐 CRITICAL",
                        SecurityAccess => "🔓 ACCESS",
                    })
                    .unwrap_or_else(|| match event.level {
                        Level::ERROR => ERROR,
                        Level::WARN => WARN,
                        Level::INFO => INFO,
                        Level::DEBUG => DEBUG,
                        Level::TRACE => TRACE,
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
                    .unwrap_or("");

                write!(writer, "{} {:<11} ", timestamp_fmt, level_fmt)?;

                indent
                    .iter()
                    .try_for_each(|fill| write!(writer, "{}", fill))?;

                writeln!(writer, "[{}]: {}", tag_fmt, event.message)
            }
            MyProcessedEventOrSpan::Span(span) => {
                let timestamp_fmt = span.timestamp.format("%b %m %H:%M:%S.%3f");

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

                write!(writer, "{} 📍 SPAN      ", timestamp_fmt)?;

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
                        fmt_rec(event, indent, writer)?;
                    }

                    // Last child, set to Turn
                    indent.last_mut().map(|fill| *fill = Turn);
                    fmt_rec(last, indent, writer)?;

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
    fmt_rec(&processed_logs, &mut indent, &mut writer).expect("Write failed");
    writer
}
