//! Code generation engine for the `fack` derive language.
//!
//! The public API keeps compiler representations opaque while exposing the
//! parse, validate, and expansion stages required by downstream tooling.
#![no_std]

extern crate alloc;

mod expand;
mod input;
mod semantic;

/// A syntactically coherent error derive target before semantic validation.
#[derive(Clone, Debug)]
// NOTE(invariant): Construction succeeds only after syntax parsing and declaration classification complete.
pub struct Target(input::Target);

impl Target {
    /// Parse one derive input into a syntactically coherent target.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when syntax level requirements are not satisfied.
    #[inline]
    pub fn input(input: &syn::DeriveInput) -> syn::Result<Self> {
        input::Target::parse(input).map(Self)
    }

    /// Validate field dependent semantics for this target.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when a field dependent requirement is not satisfied.
    #[inline]
    pub fn validate(self) -> syn::Result<ValidatedTarget> {
        let Self(target) = self;

        semantic::Target::from_input(target).map(ValidatedTarget)
    }
}

/// A semantically validated error derive target ready for expansion.
#[derive(Clone, Debug)]
// NOTE(invariant): Construction succeeds only after every field dependent requirement resolves into semantic state.
pub struct ValidatedTarget(semantic::Target);

impl ValidatedTarget {
    /// Expand this validated target into Rust implementation tokens.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when final generated syntax cannot be constructed.
    #[inline]
    pub fn expand(self) -> syn::Result<proc_macro2::TokenStream> {
        let Self(target) = self;

        expand::target(target)
    }
}

/// Parse, validate, and expand one derive input.
///
/// # Errors
///
/// Returns the first syntax, semantic, or expansion diagnostic produced by the
/// engine.
#[inline]
pub fn generate(input: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    Target::input(input)?.validate()?.expand()
}
