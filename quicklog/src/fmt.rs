use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Local, TimeZone, Utc,
};
use dyn_fmt::AsStrFormatExt;
use quicklog_flush::{stdout_flusher::StdoutFlusher, Flush};

#[cfg(feature = "ansi")]
use nu_ansi_term::Style;

use std::fmt::Write;

#[cfg(not(feature = "ansi"))]
use std::marker::PhantomData;

use crate::{
    level::{Level, LevelFormat},
    Metadata,
};

/// Contains data associated with each log entry.
pub struct LogContext<'a> {
    timestamp: u64,
    metadata: &'a Metadata,
    log_args: &'a [String],
}

impl<'a> LogContext<'a> {
    pub(crate) fn new(timestamp: u64, metadata: &'a Metadata, log_args: &'a [String]) -> Self {
        Self {
            timestamp,
            metadata,
            log_args,
        }
    }

    /// The UNIX timestamp that was recorded as part of the log message.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Log [`Metadata`].
    pub fn metadata(&self) -> &'a Metadata {
        self.metadata
    }

    /// Constructs full format string, with structured fields appended.
    #[inline]
    pub fn full_fmt_str(&self) -> String {
        // Construct format string for prefixed (structured) fields and append
        // to original format string
        let fields = self.metadata.fields();
        let mut fmt_str = self.metadata.format_str().to_string();
        if !fmt_str.is_empty() && !fields.is_empty() {
            fmt_str.push(' ');
        }
        let num_prefixed_fields = fields.len();
        let mut field_buf = String::new();
        for (idx, field) in fields.iter().enumerate() {
            field_buf.push_str(field);
            field_buf.push_str("={}");

            fmt_str.push_str(field_buf.as_str());
            if idx < num_prefixed_fields - 1 {
                fmt_str.push(' ');
            }

            field_buf.clear();
        }

        fmt_str
    }

    /// Formats full log message, including structured fields.
    #[inline]
    pub fn full_message(&self) -> String {
        self.full_fmt_str().format(self.log_args)
    }
}

/// Buffered writer wrapping an underlying [`Flush`] implementor.
pub struct Writer {
    buf: String,
    flusher: Box<dyn Flush>,
    #[cfg(feature = "ansi")]
    ansi: bool,
}

impl Writer {
    pub(crate) fn with_flusher(self, flusher: Box<dyn Flush>) -> Self {
        Self { flusher, ..self }
    }

    /// Writes buffer to underlying flusher.
    pub(crate) fn flush(&mut self) {
        self.flusher.flush_one(std::mem::take(&mut self.buf));
    }

    /// Writes timestamp, formatting it with ANSI colors if the `ansi` feature
    /// is on and ANSI colors are enabled.
    fn write_timestamp<T: std::fmt::Display>(&mut self, timestamp: T) -> std::fmt::Result {
        #[cfg(feature = "ansi")]
        {
            if self.ansi {
                let dimmed = Style::new().dimmed();
                return write!(
                    self,
                    "{}[{}]{}",
                    dimmed.prefix(),
                    timestamp,
                    dimmed.suffix(),
                );
            }
        }

        write!(self, "[{}]", timestamp)
    }

    /// Writes log level, formatting it with ANSI colors if the `ansi` feature
    /// is on and ANSI colors are enabled.
    fn write_level(&mut self, level: Level) -> std::fmt::Result {
        #[cfg(feature = "ansi")]
        {
            if self.ansi {
                let dimmed = Style::new().dimmed();
                return write!(
                    self,
                    "{}{}{}",
                    dimmed.paint("["),
                    LevelFormat::new(level, self.ansi),
                    dimmed.paint("]")
                );
            }

            write!(self, "[{}]", LevelFormat::new(level, false))
        }

        #[cfg(not(feature = "ansi"))]
        write!(self, "[{}]", LevelFormat::new(level))
    }

    /// Clears write buffer.
    pub(crate) fn clear(&mut self) {
        self.buf.clear();
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self {
            buf: String::new(),
            flusher: Box::new(StdoutFlusher),
            #[cfg(feature = "ansi")]
            ansi: false,
        }
    }
}

impl std::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buf.write_str(s)
    }
}

/// Customize format output as desired.
///
/// # Examples
///
/// ```no_run
/// use chrono::{DateTime, Utc};
/// use quicklog::{
///     config,
///     fmt::{LogContext, PatternFormatter, Writer},
///     init, Metadata,
/// };
///
/// use std::fmt::Write;
///
/// struct MyFormatter {
///     callsite: &'static str,
/// }
///
/// impl PatternFormatter for MyFormatter {
///     fn custom_format(&self, ctx: LogContext<'_>, writer: &mut Writer) -> std::fmt::Result {
///         writeln!(
///             writer,
///             "[CALLSITE: {}][{:?}][{}]{}",
///             self.callsite,
///             ctx.timestamp(),
///             ctx.metadata().level(),
///             ctx.full_message(),
///         )
///     }
/// }
///
/// # fn main() {
/// let my_formatter = MyFormatter {
///     callsite: "main callsite",
/// };
/// init!(config().formatter(my_formatter));
/// // logging calls...
/// # }
/// ```
pub trait PatternFormatter {
    /// Specifies how to format the log output, given the formatted log record
    /// and other metadata.
    fn custom_format(&self, ctx: LogContext<'_>, writer: &mut Writer) -> std::fmt::Result;
}

/// Formats logs in JSON output.
///
/// Only logs timestamp and log level by default.
///
/// # Example
///
/// ```no_run
/// # use quicklog::{config, formatter, info, init};
/// # fn main() {
/// init!(config().formatter(formatter().json().build()));
///
/// // {"timestamp":"1706065336","level":"INF","fields":{"message":"some message: 5","hello": "123","world":"there"}}
/// info!(hello = "123", world = "there", "some message: {}", 5);
/// # }
/// ```
pub struct JsonFormatter<Tz: TimeZone> {
    target: bool,
    filename: bool,
    line: bool,
    level: bool,
    timestamp: Timestamp<Tz>,
}

impl Default for JsonFormatter<Utc> {
    fn default() -> Self {
        Self {
            target: false,
            filename: false,
            line: false,
            level: true,
            timestamp: Timestamp::default(),
        }
    }
}

impl<Tz: TimeZone> PatternFormatter for JsonFormatter<Tz>
where
    Tz::Offset: std::fmt::Display,
{
    fn custom_format(&self, ctx: LogContext<'_>, writer: &mut Writer) -> std::fmt::Result {
        write!(writer, "{{")?;

        // Indicate whether following fields should prepend comma
        let mut has_previous = false;
        let time = self.timestamp.format_timestamp(ctx.timestamp)?;
        if let Some(t) = time {
            write!(writer, "\"timestamp\": \"{}\"", t)?;
        }

        if self.level {
            if has_previous {
                write!(writer, ",")?;
            } else {
                has_previous = true;
            }

            write!(writer, "\"level\": \"{}\"", ctx.metadata.level())?;
        }

        if self.filename {
            if has_previous {
                write!(writer, ",")?;
            } else {
                has_previous = true;
            }

            write!(writer, "\"filename\": \"{}\"", ctx.metadata.file())?;
        }

        if self.target {
            if has_previous {
                write!(writer, ",")?;
            } else {
                has_previous = true;
            }

            write!(writer, "\"filename\": \"{}\"", ctx.metadata.target())?;
        }

        if self.line {
            if has_previous {
                write!(writer, ",")?;
            } else {
                has_previous = true;
            }

            write!(writer, "\"filename\": \"{}\"", ctx.metadata.line())?;
        }

        // Not possible to log empty message, so will always have at least one field
        if has_previous {
            write!(writer, ",")?;
        }
        write!(writer, "\"fields\":{{")?;

        let num_field_args = ctx.metadata.fields().len();
        let all_args = ctx.log_args;
        debug_assert!(all_args.len() >= num_field_args);

        let end_idx = num_field_args.min(all_args.len());
        let field_start_idx = all_args.len() - end_idx;
        let fields_args = &ctx.log_args[field_start_idx..];
        let fmt_args = &ctx.log_args[..field_start_idx];

        let fmt_str = ctx.metadata.format_str();
        let has_fmt_str = !fmt_str.is_empty();
        if has_fmt_str {
            write!(writer, "\"message\":\"{}\"", fmt_str.format(fmt_args))?;
        }

        if !fields_args.is_empty() {
            if has_fmt_str {
                write!(writer, ",")?;
            }
            for (idx, (name, arg)) in ctx
                .metadata
                .fields()
                .iter()
                .zip(fields_args.iter())
                .enumerate()
            {
                write!(writer, "\"{}\":\"{}\"", name, arg)?;

                if idx < num_field_args - 1 {
                    write!(writer, ",")?;
                }
            }
        }

        // Extra closing brace to end "fields"
        writeln!(writer, "}}}}")
    }
}

struct Timestamp<Tz> {
    inner: TimestampImp<Tz>,
    display_timestamp: bool,
}

impl<Tz: TimeZone> Timestamp<Tz>
where
    Tz::Offset: std::fmt::Display,
{
    fn format_timestamp<'a>(
        &self,
        timestamp: u64,
    ) -> Result<Option<DelayedFormat<StrftimeItems<'a>>>, std::fmt::Error> {
        if !self.display_timestamp {
            return Ok(None);
        };

        let TimestampImp {
            format: TimestampFormat(format),
            tz,
        } = &self.inner;

        let secs = timestamp / 1_000_000_000;
        let nsecs = timestamp - secs * 1_000_000_000;
        let dt = DateTime::from_timestamp(secs as i64, nsecs as u32)
            .ok_or(std::fmt::Error)?
            .with_timezone(tz);

        Ok(Some(dt.format(format)))
    }
}

impl Default for Timestamp<Utc> {
    fn default() -> Self {
        Self {
            inner: TimestampImp::default(),
            display_timestamp: true,
        }
    }
}

struct TimestampImp<Tz> {
    format: TimestampFormat,
    tz: Tz,
}

impl Default for TimestampImp<Utc> {
    fn default() -> Self {
        Self {
            format: TimestampFormat::default(),
            tz: Utc,
        }
    }
}

#[derive(Copy, Clone)]
struct TimestampFormat(&'static str);

impl Default for TimestampFormat {
    fn default() -> Self {
        Self("%s")
    }
}

/// A basic formatter implementing [`PatternFormatter`].
pub struct QuickLogFormatter<Tz> {
    target: bool,
    filename: bool,
    line: bool,
    level: bool,
    timestamp: Timestamp<Tz>,
    #[cfg(feature = "ansi")]
    ansi: bool,
}

impl<Tz: TimeZone> QuickLogFormatter<Tz>
where
    Tz::Offset: std::fmt::Display,
{
    /// Formats timestamp if it is enabled.
    fn format_timestamp(&self, timestamp: u64, writer: &mut Writer) -> std::fmt::Result {
        let time = self.timestamp.format_timestamp(timestamp)?;

        if let Some(t) = time {
            writer.write_timestamp(t)?;
        }

        Ok(())
    }

    /// Formats log level if it is enabled.
    fn format_level(&self, level: Level, writer: &mut Writer) -> std::fmt::Result {
        if !self.level {
            return Ok(());
        }

        writer.write_level(level)
    }

    /// Formats remaining metadata-related information and log message.
    #[cfg(feature = "ansi")]
    fn format_metadata_and_msg(
        &self,
        ctx: LogContext<'_>,
        writer: &mut Writer,
    ) -> std::fmt::Result {
        let dimmed = self
            .ansi
            .then(|| Style::new().dimmed())
            .unwrap_or_else(Style::new);

        if self.filename {
            write!(
                writer,
                "{}{}{}",
                dimmed.paint(ctx.metadata.file()),
                dimmed.paint(":"),
                if self.target { "" } else { " " }
            )?;
        }

        let line_number = self.line.then_some(ctx.metadata.line());
        if self.target {
            write!(
                writer,
                "{}{}{}",
                dimmed.paint(ctx.metadata.target()),
                dimmed.paint(":"),
                if line_number.is_some() { "" } else { " " }
            )?;
        }

        if let Some(n) = line_number {
            write!(writer, "{}{}:{}", dimmed.prefix(), n, dimmed.suffix())?;
        }

        writeln!(writer, "{}", ctx.full_message())
    }

    /// Formats remaining metadata-related information and log message.
    #[cfg(not(feature = "ansi"))]
    fn format_metadata_and_msg(
        &self,
        ctx: LogContext<'_>,
        writer: &mut Writer,
    ) -> std::fmt::Result {
        if self.filename {
            write!(
                writer,
                "{}:{}",
                ctx.metadata.file(),
                if self.target { "" } else { " " }
            )?;
        }

        let line_number = self.line.then_some(ctx.metadata.line());
        if self.target {
            write!(
                writer,
                "{}:{}",
                ctx.metadata.target(),
                if line_number.is_some() { "" } else { " " }
            )?;
        }

        if let Some(n) = line_number {
            write!(writer, "{}:", n)?;
        }

        writeln!(writer, "{}", ctx.full_message())
    }
}

/// Default format.
pub struct Normal {
    #[cfg(feature = "ansi")]
    ansi: bool,
}

/// JSON format.
pub struct Json;

/// Configuration builder.
pub struct FormatterBuilder<F, Tz> {
    target: bool,
    filename: bool,
    line: bool,
    level: bool,
    timestamp: Timestamp<Tz>,
    #[cfg(feature = "ansi")]
    format: F,
    #[cfg(not(feature = "ansi"))]
    format: PhantomData<F>,
}

impl<F, Tz: TimeZone> FormatterBuilder<F, Tz>
where
    Tz::Offset: std::fmt::Display,
{
    /// Toggles whether to print module path.
    pub fn with_target(self, target: bool) -> Self {
        Self { target, ..self }
    }

    /// Toggles whether to print filename.
    pub fn with_filename(self, filename: bool) -> Self {
        Self { filename, ..self }
    }

    /// Toggles whether to print line number.
    pub fn with_line(self, line: bool) -> Self {
        Self { line, ..self }
    }

    /// Toggles whether to print log level.
    pub fn with_level(self, level: bool) -> Self {
        Self { level, ..self }
    }

    /// Enables display of timestamp.
    ///
    /// Overrides default timestamp representation to nanoseconds since Unix
    /// epoch.
    pub fn with_time(self) -> FormatterBuilder<F, Utc> {
        FormatterBuilder {
            timestamp: Timestamp::default(),
            target: self.target,
            filename: self.filename,
            line: self.line,
            level: self.level,
            format: self.format,
        }
    }

    /// Describes how to format timestamp.
    ///
    /// This follows the format supported by
    /// [`strftime`](chrono::format::strftime).
    pub fn with_time_fmt(self, fmt: &'static str) -> Self {
        Self {
            timestamp: Timestamp {
                inner: TimestampImp {
                    format: TimestampFormat(fmt),
                    ..self.timestamp.inner
                },
                display_timestamp: true,
            },
            ..self
        }
    }

    pub fn with_time_local(self) -> FormatterBuilder<F, Local> {
        FormatterBuilder {
            timestamp: Timestamp {
                inner: TimestampImp {
                    format: TimestampFormat(self.timestamp.inner.format.0),
                    tz: Local,
                },
                display_timestamp: true,
            },
            target: self.target,
            filename: self.filename,
            line: self.line,
            level: self.level,
            format: self.format,
        }
    }

    pub fn with_time_utc(self) -> FormatterBuilder<F, Utc> {
        FormatterBuilder {
            timestamp: Timestamp {
                inner: TimestampImp {
                    format: self.timestamp.inner.format,
                    tz: Utc,
                },
                display_timestamp: true,
            },
            target: self.target,
            filename: self.filename,
            line: self.line,
            level: self.level,
            format: self.format,
        }
    }

    /// Disable display of timestamp.
    pub fn without_time(self) -> Self {
        Self {
            timestamp: Timestamp {
                inner: self.timestamp.inner,
                display_timestamp: false,
            },
            ..self
        }
    }
}

impl<Tz: TimeZone + 'static> FormatterBuilder<Normal, Tz>
where
    Tz::Offset: std::fmt::Display,
{
    /// Toggles whether to enable ANSI formatting.
    pub fn with_ansi(self, ansi: bool) -> Self {
        #[cfg(not(feature = "ansi"))]
        {
            if ansi {
                eprintln!(
                "Called `with_ansi(true)` but `ansi` feature not enabled; this setting will be ignored."
            );
            }

            self
        }

        #[cfg(feature = "ansi")]
        {
            Self {
                format: Normal { ansi },
                ..self
            }
        }
    }

    /// Transforms the underlying format to use JSON formatting.
    pub fn json(self) -> FormatterBuilder<Json, Tz> {
        FormatterBuilder {
            target: self.target,
            filename: self.filename,
            line: self.line,
            level: self.level,
            timestamp: self.timestamp,
            #[cfg(feature = "ansi")]
            format: Json,
            #[cfg(not(feature = "ansi"))]
            format: PhantomData,
        }
    }

    pub fn build(self) -> QuickLogFormatter<Tz> {
        QuickLogFormatter {
            target: self.target,
            filename: self.filename,
            line: self.line,
            level: self.level,
            timestamp: self.timestamp,
            #[cfg(feature = "ansi")]
            ansi: self.format.ansi,
        }
    }
}

impl<Tz: TimeZone + 'static> FormatterBuilder<Json, Tz>
where
    Tz::Offset: std::fmt::Display,
{
    pub fn build(self) -> JsonFormatter<Tz> {
        JsonFormatter {
            target: self.target,
            filename: self.filename,
            line: self.line,
            level: self.level,
            timestamp: self.timestamp,
        }
    }
}

impl Default for FormatterBuilder<Normal, Utc> {
    fn default() -> Self {
        Self {
            target: false,
            filename: false,
            line: false,
            level: true,
            timestamp: Timestamp::default(),
            #[cfg(feature = "ansi")]
            format: Normal { ansi: true },
            #[cfg(not(feature = "ansi"))]
            format: PhantomData,
        }
    }
}

impl<Tz: TimeZone> PatternFormatter for QuickLogFormatter<Tz>
where
    Tz::Offset: std::fmt::Display,
{
    fn custom_format(&self, ctx: LogContext<'_>, writer: &mut Writer) -> std::fmt::Result {
        #[cfg(feature = "ansi")]
        {
            writer.ansi = self.ansi;
        }

        self.format_timestamp(ctx.timestamp, writer)?;
        self.format_level(ctx.metadata.level(), writer)?;

        self.format_metadata_and_msg(ctx, writer)
    }
}

/// Configures the global formatter.
///
/// The default formatting behavior can be changed by setting up and then
/// initializing [`FormatterBuilder`].
///
/// # Examples
///
/// Only display log message:
///
/// ```rust
/// # fn main() {
/// let formatter = quicklog::formatter()
///     .without_time() // don't output timestamp
///     .with_level(false) // don't output level
///     .build();
/// quicklog::init!(quicklog::config().formatter(formatter));
/// # }
/// ```
///
/// Format time as ISO 8601 format: (see [`chrono` strftime
/// docs](https://docs.rs/chrono/0.4.31/chrono/format/strftime/index.html) for
/// syntax options).
///
/// ```rust
/// # fn main() {
/// let formatter = quicklog::formatter()
///     .with_time_fmt("%+") // ISO 8601 format
///     .build();
/// quicklog::init!(quicklog::config().formatter(formatter));
/// # }
/// ```
#[inline]
pub fn formatter() -> FormatterBuilder<Normal, Utc> {
    FormatterBuilder::default()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::Timestamp;

    #[test]
    fn default_timestamp_fmt() {
        let ts = Timestamp::default();
        let now = Utc::now()
            .timestamp_nanos_opt()
            .expect("unable to get current time");
        let formatted = ts
            .format_timestamp(now as u64)
            .expect("failed to format timestamp")
            .expect("display timestamp not enabled by default");

        assert_eq!(format!("{}", formatted), (now / 1_000_000_000).to_string());
    }
}
