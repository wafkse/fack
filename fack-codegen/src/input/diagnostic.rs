//! Diagnostic accumulation used by syntax parsing.
//!
//! Parsing can discover independent attribute errors in one source item. This
//! accumulator preserves those diagnostics until a stage boundary decides
//! whether construction may continue.

/// A list of `syn` diagnostics accumulated before returning to the caller.
#[derive(Debug, Clone, Default)]
// NOTE(invariant): The optional error contains the combination of every
// diagnostic accumulated so far.
pub struct Errors(Option<syn::Error>);

impl Errors {
    /// Construct an empty diagnostic list.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(None)
    }

    /// Append one diagnostic.
    #[inline]
    pub fn push(&mut self, error: syn::Error) {
        let &mut Self(ref mut target) = self;

        match target.as_mut() {
            Some(existing) => existing.combine(error),
            None => *target = Some(error),
        }
    }

    /// Run the next fallible stage only when no diagnostic was accumulated.
    #[inline]
    pub fn and_then<T>(self, operation: impl FnOnce() -> syn::Result<T>) -> syn::Result<T> {
        match self {
            Self(Some(error)) => Err(error),
            Self(None) => operation(),
        }
    }

    /// Return the value when no diagnostic has been accumulated.
    #[inline]
    pub fn finish<T>(self, value: T) -> syn::Result<T> {
        match self {
            Self(Some(error)) => Err(error),
            Self(None) => Ok(value),
        }
    }
}
