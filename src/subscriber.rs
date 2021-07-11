use std::fmt;
use std::io::{self, Write as _};

use crate::timings::Stopwatch;

use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layered, SubscriberExt};
use tracing_subscriber::registry::Registry;
use tracing_subscriber::Layer;

use crate::formatter::LogFmt;

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

pub struct MyBuffer {
    buf: Vec<u8>,
}

impl Layer<Registry> for MyLayer {
    fn new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        let timed = attrs.fields().field("timed").is_some();

        if timed && extensions.get_mut::<Stopwatch>().is_none() {
            extensions.insert(Stopwatch::new());
        }

        if extensions.get_mut::<MyBuffer>().is_none() {
            let buf = MyBuffer::new(timed, span.name());
            // self.fmt.new_span(&mut buf, id, &ctx).expect("Write failed");
            extensions.insert(buf);
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        if let Some(timings) = extensions.get_mut::<Stopwatch>() {
            timings.now_busy();
        }
    }

    fn on_event(&self, event: &Event, ctx: Context<Registry>) {
        let span = match ctx.lookup_current() {
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
            .format_event(buf, event, &ctx)
            .expect("Write failed");
    }

    /*
    fn on_exit(&self, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        if let Some(timings) = extensions.get_mut::<Timings>() {
            timings.now_idle();

            eprintln!("{} exited, and was busy for {}", span.name(), timings.busy);
        } else {
            eprintln!("{} exited", span.name());
        }
    }
    */

    fn on_exit(&self, id: &Id, ctx: Context<Registry>) {
        // Problem: `on_exit` can be called multiple times,
        // which leads to a double panic because I remove the `KaniLogBuffer`.
        // But it seems like `on_close` is never called-
        // what do I do?
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        if let Some(timings) = extensions.get_mut::<Stopwatch>() {
            timings.now_idle();
        }

        let logbuf = extensions
            .get_mut::<MyBuffer>()
            .expect("Log buffer not found, this is a bug");

        // self.fmt.close_span(logbuf, id, &ctx).expect("Write failed");

        let logs = logbuf.dump();

        match span.parent() {
            Some(parent) => {
                // There exists a parent span
                // Write to parent
                let mut extensions = parent.extensions_mut();

                let logbuf = extensions
                    .get_mut::<MyBuffer>()
                    .expect("Log buffer not found, this is a bug");

                logbuf.buf.extend_from_slice(&logs[..]);
            }
            None => {
                // There is no parent span
                // Write to `stderr`
                io::stderr().write_all(&logs[..]).unwrap();
            }
        }
    }
}

impl MyBuffer {
    fn new(_timed: bool, _name: &str) -> Self {
        let res = MyBuffer {
            // TODO: use ctx to get information about what this should be
            buf: vec![],
        };
        // TODO: update this to use the ctx to get span information
        // write!(res, "INFO: {}: OPENED", name).unwrap();
        res
    }

    fn dump(&mut self) -> Vec<u8> {
        // TODO: update this to use the ctx to get span information
        // let a = self.path.clone();
        // write!(self, "INFO: {}: CLOSED", a).unwrap();
        let mut dump = Vec::new();
        std::mem::swap(&mut dump, &mut self.buf);
        dump
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
}
