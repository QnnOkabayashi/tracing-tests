use std::any::TypeId;
use std::fmt;
use std::io;

use tracing::field::{self, Field, Visit};
use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layered, SubscriberExt};
use tracing_subscriber::registry::Registry;
use tracing_subscriber::Layer;

use crate::formatter::LogFmt;
use crate::timings::Stopwatch;

pub struct MySubscriber {
    inner: Layered<MyLayer, Registry>,
}

pub struct MyLayer {
    fmt: LogFmt,
}

impl MySubscriber {
    pub fn json() -> Self {
        MySubscriber {
            inner: Registry::default().with(MyLayer { fmt: LogFmt::Json }),
        }
    }

    pub fn pretty() -> Self {
        MySubscriber {
            inner: Registry::default().with(MyLayer {
                fmt: LogFmt::Pretty,
            }),
        }
    }
}

#[derive(Default)]
pub struct MyBuffer {
    buf: Vec<u8>,
}

impl MyLayer {
    fn log_event(&self, event: &Event, ctx: &Context<Registry>) {
        let span = match ctx.event_span(event) {
            Some(span) => span,
            // We're not in any spans, do we still care about the log?
            // Let's just ignore it for now and short-circuit.
            _ => return,
        };

        // `extensions_mut` returns an `ExtensionsMut`, which is essentially a
        // wrapping for the `AnyMap` type offered in https://docs.rs/anymap/0.12.1/anymap/
        let mut extensions = span.extensions_mut();

        let buf = extensions
            .get_mut::<MyBuffer>()
            .expect("Log buffer not found, this is a bug");

        self.fmt
            .format_event(buf, event, ctx)
            .expect("Write failed");
    }
}

struct DebugStr<'a>(&'a str);

impl<'a> fmt::Debug for DebugStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! with_labeled_event {
    ($id:ident, $span:ident, $message:literal, {$($field:literal: $value:expr),*}, |$event:ident| $code:block) => {
        let meta = $span.metadata();
        let cs = meta.callsite();
        let fs = field::FieldSet::new(&["message",$($field),*], cs);
        let mut iter = fs.iter();
        let message = field::debug(crate::subscriber::DebugStr($message));
        let v = [
            (&iter.next().unwrap(), Some(&message as &dyn field::Value)),
            $(
                (&iter.next().unwrap(), Some(&$value as &dyn field::Value)),
            )*
        ];
        let vs = fs.value_set(&v);
        let $event = Event::new_child_of($id, meta, &vs);
        $code
    };
}

impl Layer<Registry> for MyLayer {
    fn new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let timed = {
            struct Visitor {
                timed: bool, // add other attrs we care about here...
            }

            impl Visit for Visitor {
                fn record_i64(&mut self, _: &Field, _: i64) {}

                fn record_u64(&mut self, _: &Field, _: u64) {}

                fn record_bool(&mut self, field: &Field, value: bool) {
                    if field.name() == "timed" {
                        self.timed = value;
                    }
                }

                fn record_str(&mut self, _: &Field, _: &str) {}

                fn record_error(&mut self, _: &Field, _: &(dyn std::error::Error + 'static)) {}

                fn record_debug(&mut self, _: &Field, _: &dyn fmt::Debug) {}
            }

            let mut visitor = Visitor { timed: false };
            attrs.record(&mut visitor);
            visitor.timed
        };

        let mut extensions = span.extensions_mut();

        if timed && extensions.get_mut::<Stopwatch>().is_none() {
            extensions.insert(Stopwatch::new());
        }

        if extensions.get_mut::<MyBuffer>().is_none() {
            let buf = MyBuffer::default();
            extensions.insert(buf);
        }

        // TODO: maybe add `FmtSpan` flags to choose which of these we want
        with_labeled_event!(id, span, "opened", {}, |event| {
            drop(extensions);
            self.log_event(&event, &ctx);
        });
    }

    fn on_event(&self, event: &Event, ctx: Context<Registry>) {
        self.log_event(event, &ctx)
    }

    fn on_enter(&self, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        if let Some(timings) = extensions.get_mut::<Stopwatch>() {
            timings.now_busy();
        }

        // TODO: maybe log how long the span was idle for? could be nice to see
        // where there were breaks in execution
    }

    fn on_exit(&self, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        if let Some(timings) = extensions.get_mut::<Stopwatch>() {
            timings.now_idle();
        }
    }

    fn on_close(&self, id: Id, ctx: Context<Registry>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();
        // `tracing_subscriber` also just calls the macro twice,
        // so at least this isn't worse than that.
        if let Some(stopwatch) = extensions.remove::<Stopwatch>() {
            // Thanks for making this private :(, guess I'll just copy paste
            // https://github.com/tokio-rs/tracing/blob/c848820fc62c274d3df1be61303d97f3b6802673/tracing-subscriber/src/fmt/format/mod.rs#L1229-L1245
            struct TimingDisplay(u64);
            impl fmt::Display for TimingDisplay {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    let mut t = self.0 as f64;
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

            let busy = field::display(TimingDisplay(stopwatch.busy()));
            let idle = field::display(TimingDisplay(stopwatch.idle()));
            with_labeled_event!(id, span, "closed", {"time.busy": busy, "time.idle": idle }, |event| {
                drop(extensions);
                self.log_event(&event, &ctx);
            });
        } else {
            with_labeled_event!(id, span, "closed", {}, |event| {
                drop(extensions);
                self.log_event(&event, &ctx);
            });
        }

        let mut extensions = span.extensions_mut();

        let buf = extensions
            .remove::<MyBuffer>()
            .expect("Log buffer not found, this is a bug");

        match span.parent() {
            Some(parent) => {
                parent
                    .extensions_mut()
                    .get_mut::<MyBuffer>()
                    .expect("Log buffer not found, this is a bug")
                    .append_child(buf);
            }
            None => {
                buf.to_writer(&mut io::stderr()).unwrap();
            }
        }
    }
}

impl MyBuffer {
    fn append_child(&mut self, mut child: Self) {
        self.buf.append(&mut child.buf)
    }

    fn to_writer(self, writer: &mut impl io::Write) -> io::Result<()> {
        writer.write_all(&self.buf[..])
    }
}

impl fmt::Write for MyBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

impl Subscriber for MySubscriber {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        self.inner.max_level_hint()
    }

    fn new_span(&self, span: &Attributes) -> Id {
        self.inner.new_span(span)
    }

    fn record(&self, span: &Id, values: &Record) {
        self.inner.record(span, values)
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.inner.record_follows_from(span, follows)
    }

    fn event(&self, event: &Event) {
        self.inner.event(event)
    }

    fn enter(&self, span: &Id) {
        self.inner.enter(span)
    }

    fn exit(&self, span: &Id) {
        self.inner.exit(span)
    }

    fn clone_span(&self, id: &Id) -> Id {
        self.inner.clone_span(id)
    }

    fn try_close(&self, id: Id) -> bool {
        self.inner.try_close(id)
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        if id == TypeId::of::<Self>() {
            Some(self as *const Self as *const ())
        } else {
            self.inner.downcast_raw(id)
        }
    }
}
