//! Code generation engine for the `fack` derive language.
//!
//! The public API keeps compiler representations opaque while exposing the
//! parse, validate, and expansion stages required by downstream tooling.
#![no_std]
#![forbid(unsafe_code, missing_docs, rustdoc::all)]

extern crate alloc;

mod binding;
mod bounds;
mod diagnostic;
mod enumerate;
mod expand;
mod field;
mod format;
mod resolve;
mod semantics;
mod source;
mod structure;
mod syntax;
mod target;
mod validate;

pub use target::Target;
pub use validate::ValidatedTarget;

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
