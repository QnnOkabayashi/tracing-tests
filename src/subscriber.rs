use std::any::TypeId;
use std::fmt;
use std::io;

use chrono::{DateTime, Utc};

use tokio::sync::mpsc::UnboundedSender;

use tracing::field::{self, Field, Visit};
use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Level, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layered, SubscriberExt};
use tracing_subscriber::registry::Registry;
use tracing_subscriber::Layer;

use crate::formatter::LogFmt;
use crate::timings::Timings;

pub struct MySubscriber {
    inner: Layered<MyLayer, Registry>,
}

pub struct MyLayer {
    fmt: LogFmt,
    log_tx: UnboundedSender<(LogFmt, Vec<MyEvent>)>,
}

#[derive(Debug)]
pub struct MyEvent {
    pub timestamp: DateTime<Utc>,
    pub level: Level,
    pub spans: Option<Vec<&'static str>>,
    pub target: String,
    pub fields: Vec<(&'static str, String)>,
}

impl MySubscriber {
    pub fn new(fmt: LogFmt, log_tx: UnboundedSender<(LogFmt, Vec<MyEvent>)>) -> Self {
        MySubscriber {
            inner: Registry::default().with(MyLayer { fmt, log_tx }),
        }
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

impl MyLayer {
    fn log_event(&self, event: &Event, ctx: &Context<Registry>) {
        if let Some(span) = ctx.event_span(event) {
            // The event is in a span, so we should log it.
            span.extensions_mut()
                .get_mut::<Vec<MyEvent>>()
                .expect("Log buffer not found, this is a bug")
                .push(MyEvent::new(event, ctx));
        }
    }
}

macro_rules! with_labeled_event {
    ($id:ident, $span:ident, $message:literal, {$($field:literal: $value:expr),*}, |$event:ident| $code:block) => {
        let meta = $span.metadata();
        let cs = meta.callsite();
        let fs = field::FieldSet::new(&["message",$($field),*], cs);
        let mut iter = fs.iter();
        let message = field::display($message);
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
                fn record_bool(&mut self, field: &Field, value: bool) {
                    if field.name() == "timed" {
                        self.timed = value;
                    }
                }

                fn record_debug(&mut self, _: &Field, _: &dyn fmt::Debug) {}
            }

            let mut visitor = Visitor { timed: false };
            attrs.record(&mut visitor);
            visitor.timed
        };

        let mut extensions = span.extensions_mut();

        if timed && extensions.get_mut::<Timings>().is_none() {
            extensions.insert(Timings::new());
        }

        if extensions.get_mut::<Vec<MyEvent>>().is_none() {
            // arbitrarily chosen values, I'm open to feedback
            let capacity = match *attrs.metadata().level() {
                Level::TRACE => 512,
                Level::DEBUG => 256,
                Level::INFO => 128,
                Level::WARN => 32,
                Level::ERROR => 8,
            };

            extensions.insert(Vec::<MyEvent>::with_capacity(capacity));
        }

        // TODO: maybe add `FmtSpan` flags to choose which of these we want
        with_labeled_event!(id, span, "[opened]", {}, |event| {
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

        if let Some(timings) = extensions.get_mut::<Timings>() {
            timings.now_busy();
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        if let Some(timings) = extensions.get_mut::<Timings>() {
            timings.now_idle();
        }
    }

    fn on_close(&self, id: Id, ctx: Context<Registry>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();
        // `tracing_subscriber` also just calls the macro twice,
        // so at least this isn't worse than that.
        if let Some(timings) = extensions.remove::<Timings>() {
            let busy = timings.display_busy();
            let idle = timings.display_idle();
            with_labeled_event!(id, span, "[closed]", { "time.busy": busy, "time.idle": idle }, |event| {
                drop(extensions);
                self.log_event(&event, &ctx);
            });
        } else {
            with_labeled_event!(id, span, "[closed]", {}, |event| {
                drop(extensions);
                self.log_event(&event, &ctx);
            });
        }

        let mut extensions = span.extensions_mut();

        let event_buffer = extensions
            .remove::<Vec<MyEvent>>()
            .expect("Log buffer not found, this is a bug");

        match span.parent() {
            Some(parent) => {
                parent
                    .extensions_mut()
                    .get_mut::<Vec<MyEvent>>()
                    .expect("Log buffer not found, this is a bug")
                    .extend(event_buffer);
            }
            None => {
                self.log_tx
                    .send((self.fmt, event_buffer))
                    .expect("failed to write logs");
                // let logs = self.fmt.format_events(event_buffer).expect("Write failed");

                // eprint!("{}", logs);
            }
        }
    }
}

impl MyEvent {
    fn new(event: &Event, ctx: &Context<Registry>) -> Self {
        let timestamp = Utc::now();
        let level = *event.metadata().level();
        let spans = ctx
            .event_scope(event)
            .map(|scope| scope.map(|span| span.name()).collect());
        let target = event.metadata().target().to_string();

        let mut fields = vec![];

        struct Visitor<'a>(&'a mut Vec<(&'static str, String)>);

        impl<'a> Visit for Visitor<'a> {
            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                self.0.push((field.name(), format!("{:?}", value)));
            }
        }

        event.record(&mut Visitor(&mut fields));

        MyEvent {
            timestamp,
            level,
            spans,
            target,
            fields,
        }
    }

    pub(crate) fn format_size(&self, value_pad: usize, span_pad: usize) -> usize {
        let span_size = self
            .spans
            .as_ref()
            .map(|v| {
                // length of span name + padding by formatter
                v.iter().map(|span| span.len() + span_pad).sum::<usize>()
            })
            .unwrap_or(0);

        let fields_size = self
            .fields
            .iter()
            .map(|(field, value)| {
                // length of field-value pair + padding by formatter
                field.len() + value.len() + value_pad
            })
            // length of "message" is accounted for in formatters already
            .sum::<usize>()
            - "message".len();

        let target_size = self.target.len();

        span_size + fields_size + target_size
    }
}
