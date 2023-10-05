use syn::Expr;

/// Trait for tokens to describe how they should be applied as part of a
/// format string.
pub(crate) trait FormatArg {
    /// Describes how to apply this object as part of a format string.
    /// e.g. `{:?}`, `{}`, `custom.name={}`
    fn formatter(&self) -> &'static str;
}

impl FormatArg for Expr {
    fn formatter(&self) -> &'static str {
        "{}"
    }
}
