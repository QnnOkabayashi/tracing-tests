use chrono::{DateTime, Utc};
use serde::Serialize;
use std::io;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::timings::Timings;

#[derive(Serialize)]
enum Tag {
    None,
    Admin,
}

struct Log<'a> {
    timestamp: DateTime<Utc>,
    level: Level,
    message: &'a str,
    timings: Option<&'a Timings>,
    tag: Tag,
}

impl<'a> Serialize for Log<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 4;
        if self.timings.is_some() {
            len += 1;
        }

        let mut state = serializer.serialize_struct("Event", len)?;
        // is there a way we can make this not have to go to a string first?
        state.serialize_field(
            "timestamp",
            &self.timestamp.format("%b %m %H:%M:%S.%3f").to_string(),
        )?;
        state.serialize_field(
            "level",
            match self.level {
                Level::TRACE => "TRACE",
                Level::DEBUG => "DEBUG",
                Level::INFO => "INFO",
                Level::WARN => "WARN",
                Level::ERROR => "ERROR",
            },
        )?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("tag", &self.tag)?;

        if let Some(ref timings) = self.timings {
            state.serialize_field("time", timings)?;
        }

        state.end()
    }
}

impl<'a> Log<'a> {
    pub fn write_json<W: io::Write>(&self, writer: &mut W) {
        serde_json::to_writer(writer, self).expect("Unexpected serialization failure")
    }

    pub fn write_pretty<W: io::Write>(&self, writer: &mut W) {
        write!(
            writer,
            "{time} {lvl}: {msg} {fields}",
            time = self.timestamp.format("%b %m %H:%M:%S.%3f"),
            lvl = self.level,
            msg = self.message,
            fields = 1,
        )
        .expect("Unexpected write failure")
    }
}

#[test]
fn ser_level() {
    let timings = Timings::default();

    let event = Log {
        timestamp: Utc::now(),
        level: Level::ERROR,
        message: "enter",
        timings: Some(&timings),
        tag: Tag::Admin,
    };

    event.write_json(&mut io::stdout());
    println!();
    event.write_pretty(&mut io::stdout());
    println!();
}

#[test]
fn datetime() {
    use chrono;

    let now = chrono::Utc::now().format("%b %m %H:%M:%S.%3f").to_string();

    println!("{:?}", now);
}
