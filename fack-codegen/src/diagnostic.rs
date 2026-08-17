//! Diagnostic accumulation used by syntax parsing.

/// A list of `syn` diagnostics accumulated before returning to the caller.
#[derive(Debug, Clone, Default)]
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
        let Self(target) = self;

        match target.as_mut() {
            Some(existing) => existing.combine(error),
            None => *target = Some(error),
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
