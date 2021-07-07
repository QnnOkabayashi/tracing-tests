use std::fmt::{self, Write as _};
use std::io::{self, Write as _};

use crate::timings::Stopwatch;

use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Metadata, Subscriber};
use tracing_subscriber::fmt::format::JsonFields;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::layer::{Context, Layered, SubscriberExt};
use tracing_subscriber::registry::Registry;
use tracing_subscriber::Layer;

pub struct MySubscriber {
    inner: Layered<MyLayer, Registry>,
}

pub struct MyLayer;

impl MySubscriber {
    pub fn new() -> Self {
        MySubscriber {
            inner: Registry::default().with(MyLayer),
        }
    }
}

pub struct MyBuffer {
    path: String, // the string containing the spans it's wrapped in?
    // this is poorly designed but hopefully works
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
            extensions.insert(MyBuffer::new(timed, span.name()));
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

        // I think making a new `JsonFields` each call to event basically compiles down to nothing
        JsonFields::new().format_fields(buf, event).unwrap();

        // The lifetime requires that the `fmt::Write` object must live as long
        // as the actual formatter itself. This is super fucking annoying.
        // Two options:
        // 1. We make the buffer somehow live as long as the `KaniLayer` instance.
        // 2. We make each span have its own `FormatFields`.
        //
        // Option 1 sucks, because then I have to do interior mutability and that's a hard no from me
        // Option 2 is kinda sus, but `FormatFields`s are usually ZST's, so it's not even that bad. It just
        // means that every single span will have to be assigned a formatter.
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

        let logs = extensions
            .get_mut::<MyBuffer>()
            .expect("Log buffer not found, this is a bug")
            .dump();

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
    fn new(timed: bool, name: &str) -> Self {
        let res = MyBuffer {
            // TODO: use ctx to get information about what this should be
            path: format!("{} (timed = {})", name, timed),
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

#[test]
fn test1() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        tracing::info_span!("wrapper span").in_scope(|| {
            tracing::error!("a big error!");
        });
        tracing::trace!("lol silly trace");
    })
}
