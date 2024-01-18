use chrono::{DateTime, Utc};

use crate::Metadata;

/// Customize format output as desired.
///
/// # Examples
///
/// ```no_run
/// use chrono::{DateTime, Utc};
/// use quicklog::{formatter::PatternFormatter, init, with_formatter, Metadata};
///
/// struct MyFormatter {
///     callsite: &'static str,
/// }
///
/// impl PatternFormatter for MyFormatter {
///     fn custom_format(
///         &mut self,
///         time: DateTime<Utc>,
///         metadata: &Metadata,
///         _: &[String],
///         log_record: &str,
///     ) -> String {
///         format!(
///             "[CALLSITE: {}][{:?}][{}]{}\n",
///             self.callsite, time, metadata.level, log_record
///         )
///     }
/// }
///
/// # fn main() {
/// init!();
/// let my_formatter = MyFormatter {
///     callsite: "main callsite",
/// };
/// with_formatter!(my_formatter);
/// // logging calls...
/// # }
/// ```
pub trait PatternFormatter {
    /// Specifies how to format the log output, given the formatted log record
    /// and other metadata.
    fn custom_format(
        &mut self,
        time: DateTime<Utc>,
        metadata: &Metadata,
        field_args: &[String],
        log_record: &str,
    ) -> String;

    /// Whether the formatter assumes that the structured (prefixed) fields have
    /// already been formatted as part of the final log record.
    ///
    /// For instance, if this is true, then a log like: `info!(a = 1, b = 2,
    /// "hello world)` will yield a `log_record` of "hello world a=1 b=2".
    /// Otherwise, it will simply pass a `log_record` of "hello world" to
    /// `custom_format`.
    #[inline(always)]
    fn include_structured_fields(&self) -> bool {
        true
    }
}

/// A basic formatter implementing [`PatternFormatter`].
pub struct QuickLogFormatter;

impl PatternFormatter for QuickLogFormatter {
    fn custom_format(
        &mut self,
        time: DateTime<Utc>,
        metadata: &Metadata,
        _: &[String],
        log_record: &str,
    ) -> String {
        format!(
            "[{}][{}]{}\n",
            time.format("%FT%H:%M:%S%.9f%z"),
            metadata.level,
            log_record
        )
    }
}

/// Formats logs in JSON output.
///
/// # Example
///
/// ```no_run
/// # use quicklog::{info, init, with_formatter, formatter::JsonFormatter};
/// # fn main() {
/// init!();
/// with_formatter!(JsonFormatter);
///
/// // {"timestamp":"2023-12-13T03:01:14.131540000+0000","level":"INF","fields":{"message":"some message: 5","hello": "123","world":"there"}}
/// info!(hello = "123", world = "there", "some message: {}", 5);
/// # }
/// ```
pub struct JsonFormatter;

impl PatternFormatter for JsonFormatter {
    fn custom_format(
        &mut self,
        time: DateTime<Utc>,
        metadata: &Metadata,
        fields_args: &[String],
        log_record: &str,
    ) -> String {
        let mut final_str = format!(
            "{{\"timestamp\":\"{}\",\"level\":\"{}\"",
            time.format("%FT%H:%M:%S%.9f%z"),
            metadata.level
        );

        let log_empty = log_record.is_empty();
        let num_fields = metadata.fields.len();
        let fields_empty = num_fields == 0;
        if log_empty && fields_empty {
            final_str.push('}');
            return final_str;
        }

        final_str.push_str(",\"fields\":{");
        if !log_empty {
            final_str.push_str("\"message\":\"");
            final_str.push_str(log_record);
            final_str.push('"');
        }

        if !fields_empty {
            if !log_empty {
                final_str.push(',');
            }

            for (idx, (name, arg)) in metadata.fields.iter().zip(fields_args.iter()).enumerate() {
                final_str.push('"');
                final_str.push_str(name);
                final_str.push_str("\":\"");
                final_str.push_str(arg);
                final_str.push('"');

                if idx < num_fields - 1 {
                    final_str.push(',');
                }
            }
        }
        final_str.push_str("}}\n");

        final_str
    }

    #[inline(always)]
    fn include_structured_fields(&self) -> bool {
        false
    }
}

pub(crate) fn construct_full_fmt_str(fmt_str: &str, fields: &[&str]) -> String {
    // Construct format string for prefixed (structured) fields and append
    // to original format string
    let mut fmt_str = fmt_str.to_string();
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
