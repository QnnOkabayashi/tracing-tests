use std::any::TypeId;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Write as _};
use std::time::Duration;

use chrono::{DateTime, Utc};

use tokio::sync::mpsc::UnboundedSender;

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Level, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layered, SubscriberExt};
use tracing_subscriber::registry::Registry;
use tracing_subscriber::Layer;

use crate::formatter::LogFmt;
use crate::timings::Timer;

pub struct MySubscriber {
    inner: Layered<MyLayer, Registry>,
}

pub struct MyLayer {
    fmt: LogFmt,
    log_tx: UnboundedSender<(LogFmt, MyEventOrSpan)>,
}

#[derive(Debug)]
pub struct MyEvent {
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub level: Level,
    pub tag: Option<MyEventTag>,
    pub spans: Vec<&'static str>,
}

#[derive(Debug)]
pub struct MySpanBuf {
    pub timestamp: DateTime<Utc>,
    pub name: &'static str,
    pub buf: Vec<MyEventOrSpan>,
}

#[derive(Debug)]
pub enum MyEventOrSpan {
    Event(MyEvent),
    SpanBuf(MySpanBuf, Duration),
}

#[derive(Debug)]
pub enum MyEventTag {
    AdminError,
    AdminWarn,
    AdminInfo,
    RequestError,
    RequestWarn,
    RequestInfo,
    RequestTrace,
    SecurityCritical,
    SecurityInfo,
    SecurityAccess,
    FilterError,
    FilterWarn,
    FilterInfo,
    FilterTrace,
    PerfTrace,
}

pub struct MyProcessedSpan {
    pub timestamp: DateTime<Utc>,
    pub name: &'static str,
    pub duration: f64,
    pub direct_load: Option<f64>, // If the span has no child spans, this is `None`
    pub total_load: f64,
    pub processed_buf: Vec<MyProcessedEventOrSpan>,
}

pub enum MyProcessedEventOrSpan {
    Event(MyEvent),
    Span(MyProcessedSpan),
}

impl MySubscriber {
    pub fn new(fmt: LogFmt, log_tx: UnboundedSender<(LogFmt, MyEventOrSpan)>) -> Self {
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

impl Layer<Registry> for MyLayer {
    fn new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let name = attrs.metadata().name();

        let mut extensions = span.extensions_mut();

        extensions.insert(MySpanBuf::new(name));
        extensions.insert(Timer::new());
    }

    fn on_event(&self, event: &Event, ctx: Context<Registry>) {
        if let Some(span) = ctx.event_span(event) {
            // The event is in a span, so we should log it.
            span.extensions_mut()
                .get_mut::<MySpanBuf>()
                .expect("Log buffer not found, this is a bug")
                .log_event(MyEvent::new(event, &ctx));
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<Registry>) {
        ctx.span(id)
            .expect("Span not found, this is a bug")
            .extensions_mut()
            .get_mut::<Timer>()
            .expect("Timer not found, this is a bug")
            .unpause();
    }

    fn on_exit(&self, id: &Id, ctx: Context<Registry>) {
        ctx.span(id)
            .expect("Span not found, this is a bug")
            .extensions_mut()
            .get_mut::<Timer>()
            .expect("Timer not found, this is a bug")
            .pause();
    }

    fn on_close(&self, id: Id, ctx: Context<Registry>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");

        let mut extensions = span.extensions_mut();

        let span_buf = extensions
            .remove::<MySpanBuf>()
            .expect("Span buffer not found, this is a bug");

        let duration = extensions
            .remove::<Timer>()
            .expect("Timer not found, this is a bug")
            .duration();

        match span.parent() {
            Some(parent) => {
                parent
                    .extensions_mut()
                    .get_mut::<MySpanBuf>()
                    .expect("Span buffer not found, this is a bug")
                    .log_span(span_buf, duration);
            }
            None => {
                // TODO: fix writing
                self.log_tx
                    .send((self.fmt, MyEventOrSpan::SpanBuf(span_buf, duration)))
                    .expect("failed to write logs");
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
            .map(|scope| scope.map(|span| span.name()).collect())
            .unwrap_or_else(|| vec![]);

        struct Visitor<'a>(&'a mut String, &'a mut Option<MyEventTag>);

        impl<'a> Visit for Visitor<'a> {
            fn record_u64(&mut self, field: &Field, value: u64) {
                if field.name() == "event_tag" {
                    if let Ok(tag) = value.try_into() {
                        *self.1 = Some(tag);
                    }
                }
            }

            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                if field.name() == "message" {
                    write!(self.0, "{:?}", value).expect("Write failed");
                } else {
                    todo!("Fields other than \"message\"")
                }
            }
        }

        let mut message = String::new();
        let mut tag = None;

        event.record(&mut Visitor(&mut message, &mut tag));

        MyEvent {
            timestamp,
            message,
            level,
            tag,
            spans,
        }
    }
}

impl MySpanBuf {
    fn new(name: &'static str) -> Self {
        MySpanBuf {
            timestamp: Utc::now(),
            name,
            buf: vec![],
        }
    }

    fn log_event(&mut self, event: MyEvent) {
        self.buf.push(MyEventOrSpan::Event(event));
    }

    fn log_span(&mut self, span: MySpanBuf, duration: Duration) {
        self.buf.push(MyEventOrSpan::SpanBuf(span, duration));
    }
}

impl MyEventOrSpan {
    pub fn process(self) -> MyProcessedEventOrSpan {
        fn process_rec(this: MyEventOrSpan, root_duration: Option<f64>) -> MyProcessedEventOrSpan {
            let (span_buf, duration) = match this {
                MyEventOrSpan::Event(event) => return MyProcessedEventOrSpan::Event(event),
                MyEventOrSpan::SpanBuf(span_buf, duration) => (span_buf, duration),
            };

            let duration = duration.as_nanos() as f64;

            let root_duration = root_duration.unwrap_or(duration);

            let mut processed_buf = vec![];

            let direct_load = span_buf
                .buf
                .into_iter()
                .filter_map(|event_or_span| {
                    let processed = process_rec(event_or_span, Some(root_duration));

                    let duration = match &processed {
                        MyProcessedEventOrSpan::Span(span) => Some(span.duration),
                        _ => None,
                    };

                    // Side effect: Push processed logs to processed_buf
                    processed_buf.push(processed);

                    duration
                })
                // Returns `None` if nothing comes out of the iterator, otherwise sums
                .fold(None, |sum, d| Some(d + sum.unwrap_or(0.0)))
                .map(|nested_duration| {
                    100.0 * (duration - nested_duration) / root_duration
                });

            let total_load = 100.0 * (duration / root_duration);

            MyProcessedEventOrSpan::Span(MyProcessedSpan {
                timestamp: span_buf.timestamp,
                name: span_buf.name,
                duration,
                direct_load,
                total_load,
                processed_buf,
            })
        }

        process_rec(self, None)
    }
}

impl From<MyEventTag> for u64 {
    fn from(tag: MyEventTag) -> Self {
        use MyEventTag::*;
        match tag {
            AdminError => 0,
            AdminWarn => 1,
            AdminInfo => 2,
            RequestError => 3,
            RequestWarn => 4,
            RequestInfo => 5,
            RequestTrace => 6,
            SecurityCritical => 7,
            SecurityInfo => 8,
            SecurityAccess => 9,
            FilterError => 10,
            FilterWarn => 11,
            FilterInfo => 12,
            FilterTrace => 13,
            PerfTrace => 14,
        }
    }
}

impl TryFrom<u64> for MyEventTag {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        use MyEventTag::*;
        Ok(match value {
            0 => AdminError,
            1 => AdminWarn,
            2 => AdminInfo,
            3 => RequestError,
            4 => RequestWarn,
            5 => RequestInfo,
            6 => RequestTrace,
            7 => SecurityCritical,
            8 => SecurityInfo,
            9 => SecurityAccess,
            10 => FilterError,
            11 => FilterWarn,
            12 => FilterInfo,
            13 => FilterTrace,
            14 => PerfTrace,
            _ => return Err(()),
        })
    }
}
