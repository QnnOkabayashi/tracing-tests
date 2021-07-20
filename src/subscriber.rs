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
use tracing_subscriber::registry::{Registry, SpanRef};
use tracing_subscriber::Layer;
use uuid::Uuid;

use crate::formatter::LogFmt;
use crate::timings::Timer;

pub struct MySubscriber {
    inner: Layered<MyLayer, Registry>,
}

pub struct MyLayer {
    fmt: LogFmt,
    log_tx: UnboundedSender<(LogFmt, MyLogs)>,
}

#[derive(Debug)]
pub struct MyEvent {
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub level: Level,
    pub tag: Option<MyEventTag>,
    pub spans: Vec<&'static str>,
    pub values: Vec<(&'static str, String)>,
}

#[derive(Debug)]
pub struct MySpanBuf {
    pub timestamp: DateTime<Utc>,
    pub name: &'static str,
    pub buf: Vec<MyLogs>,
    pub uuid: Option<String>, // Must convert to `fmt::Debug` object to pass field boundry
}

#[derive(Debug)]
pub enum MyLogs {
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
    pub processed_buf: Vec<MyProcessedLogs>,
    pub uuid: Option<String>,
}

pub enum MyProcessedLogs {
    Event(MyEvent),
    Span(MyProcessedSpan),
}

impl MySubscriber {
    pub fn new(fmt: LogFmt, log_tx: UnboundedSender<(LogFmt, MyLogs)>) -> Self {
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
    fn log_to_parent<'a>(&self, logs: MyLogs, parent: Option<SpanRef<'a, Registry>>) {
        match parent {
            // The parent exists- write to them
            Some(span) => span
                .extensions_mut()
                .get_mut::<MySpanBuf>()
                .expect("Log buffer not found, this is a bug")
                .log(logs),
            // The parent doesn't exist- send to formatter
            None => self
                .log_tx
                .send((self.fmt, logs))
                .expect("Failed to write logs"),
        }
    }
}

impl Layer<Registry> for MyLayer {
    fn new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<Registry>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let name = attrs.metadata().name();

        let mut uuid = None;

        attrs.record(&mut |field: &Field, value: &dyn fmt::Debug| {
            if field.name() == "uuid" {
                let mut buf = String::with_capacity(36);
                write!(&mut buf, "{:?}", value).expect("Write failed");
                uuid = Some(buf);
            }
        });

        // Take provided ID, or make a fresh one if there's no parent span.
        let uuid = uuid.or_else(|| {
            ctx.lookup_current()
                .is_none()
                .then(|| Uuid::new_v4().to_string())
        });

        let mut extensions = span.extensions_mut();

        extensions.insert(MySpanBuf::new(name, uuid));
        extensions.insert(Timer::new());
    }

    fn on_event(&self, event: &Event, ctx: Context<Registry>) {
        let logs = MyLogs::event(event, &ctx);

        self.log_to_parent(logs, ctx.event_span(event));
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

        let logs = MyLogs::SpanBuf(span_buf, duration);

        self.log_to_parent(logs, span.parent());
    }
}

impl MySpanBuf {
    fn new(name: &'static str, uuid: Option<String>) -> Self {
        MySpanBuf {
            timestamp: Utc::now(),
            name,
            buf: vec![],
            uuid,
        }
    }

    fn log(&mut self, logs: MyLogs) {
        self.buf.push(logs)
    }
}

impl MyLogs {
    fn event(event: &Event, ctx: &Context<Registry>) -> Self {
        let timestamp = Utc::now();
        let level = *event.metadata().level();
        let spans = ctx
            .event_scope(event)
            .map(|scope| scope.map(|span| span.name()).collect())
            .unwrap_or_else(|| vec![]);

        struct Visitor<'a>(
            &'a mut String,
            &'a mut Option<MyEventTag>,
            &'a mut Vec<(&'static str, String)>,
        );

        impl<'a> Visit for Visitor<'a> {
            fn record_u64(&mut self, field: &Field, value: u64) {
                if field.name() == "event_tag" {
                    let tag = value
                        .try_into()
                        .expect(&format!("Invalid `event_tag`: {}", value));
                    *self.1 = Some(tag);
                }
            }

            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                if field.name() == "message" {
                    use fmt::Write;
                    write!(self.0, "{:?}", value).expect("Write failed");
                } else {
                    self.2.push((field.name(), format!("{:?}", value)));
                }
            }
        }

        let mut message = String::new();
        let mut tag = None;
        let mut values = vec![];

        event.record(&mut Visitor(&mut message, &mut tag, &mut values));

        MyLogs::Event(MyEvent {
            timestamp,
            message,
            level,
            tag,
            spans,
            values,
        })
    }

    pub fn process(self) -> MyProcessedLogs {
        fn process_rec(logs: MyLogs, root_duration: Option<f64>) -> MyProcessedLogs {
            let (span_buf, duration) = match logs {
                MyLogs::Event(event) => return MyProcessedLogs::Event(event),
                MyLogs::SpanBuf(span_buf, duration) => (span_buf, duration),
            };

            let duration = duration.as_nanos() as f64;

            let root_duration = root_duration.unwrap_or(duration);

            let mut processed_buf = vec![];

            let direct_load = span_buf
                .buf
                .into_iter()
                .filter_map(|logs| {
                    let processed = process_rec(logs, Some(root_duration));

                    let duration = match processed {
                        MyProcessedLogs::Span(ref span) => Some(span.duration),
                        _ => None,
                    };

                    // Side effect: Push processed logs to processed_buf
                    processed_buf.push(processed);

                    duration
                })
                // Returns `None` if nothing comes out of the iterator, otherwise sums
                .fold(None, |sum_opt, child_duration| {
                    let sum = sum_opt.unwrap_or(0.0);
                    Some(sum + child_duration)
                })
                .map(|nested_duration| 100.0 * (duration - nested_duration) / root_duration);

            let total_load = 100.0 * (duration / root_duration);

            MyProcessedLogs::Span(MyProcessedSpan {
                timestamp: span_buf.timestamp,
                name: span_buf.name,
                duration,
                direct_load,
                total_load,
                processed_buf,
                uuid: span_buf.uuid,
            })
        }

        process_rec(self, None)
    }
}

// The purpose of these functions is to allow us to pass a `MyEventTag` through the fields key-value barrier,
// where `record` only allows, `i64`, `u64`, `bool`, `&str`, and `dyn fmt::Debug` values.
// We use `u64` because it's faster than `&str`, and would take the same space to transform.
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
