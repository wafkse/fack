//! Resolution from source syntax into stronger semantic identities.

/// Resolve one syntax value against explicit context.
///
/// Implementors should establish the invariant represented by `Output` once.
/// Downstream code should consume the resolved output instead of repeating the
/// original lookup or validation.
pub trait Resolve {
    /// Context required to resolve the syntax value.
    type Context;

    /// Stronger output produced after resolution succeeds.
    type Output;

    /// Resolve the value against the provided context.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the source value cannot be resolved in the
    /// supplied context.
    fn resolve(self, context: &Self::Context) -> syn::Result<Self::Output>;
}
