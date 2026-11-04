//! Procedural macro entry point for the `fack` derive language.
//!
//! This crate performs only compiler-token conversion and delegates parsing,
//! semantic validation, and expansion to `fack-codegen`. The accepted helper
//! attribute language is documented on the `Error` derive macro.

use proc_macro::TokenStream;

use fack_codegen::generate;

/// Derive `Display` and `Error` implementations from `#[error(...)]`
/// declarations.
///
/// A format string defines ordinary display behavior.
///
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error("failed to read {path}")]
/// struct ReadError {
///     path: String,
/// }
/// ```
///
/// `source(field)` selects the ordinary error source.
///
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error("network request failed")]
/// #[error(source(io))]
/// struct NetworkError {
///     io: std::io::Error,
/// }
/// ```
///
/// A transparent error declaration uses one field for the wrapper's formatting
/// and source behavior.
///
/// - `#[error(transparent)]` selects the first declared field. For tuple
///   structs and tuple variants, this is field `0`.
/// - `#[error(transparent(field))]` selects a field by name or tuple index.
///
/// The selected field must be a non-optional error. The wrapper's `Display`
/// implementation delegates formatting to that field. Its `Error::source`
/// method calls the field's `Error::source` method. Other fields may store
/// context.
///
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error(transparent)]
/// struct TupleWrapper(std::io::Error);
///
/// #[derive(Error, Debug)]
/// #[error(transparent(inner))]
/// struct Wrapper {
///     context: u8,
///     inner: std::io::Error,
/// }
/// ```
///
/// `from` requires exactly one field. It generates `From<T>` and selects that
/// field as the ordinary source.
///
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error("parse failed")]
/// #[error(from)]
/// struct ParseError(std::num::ParseIntError);
/// ```
///
/// `display(path)` selects a custom formatter function.
///
/// ```rust
/// # use fack_macro::Error;
/// fn render(error: &Rendered, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
///     let Rendered { code } = error;
///     write!(f, "rendered {code}")
/// }
///
/// #[derive(Error, Debug)]
/// #[error(display(render))]
/// struct Rendered {
///     code: u8,
/// }
/// ```
///
/// Inline control is optional. Omitting it emits no explicit inline attribute.
/// `inline` and `inline(neutral)` emit ordinary `#[inline]`. The `always` and
/// `never` strategies emit the corresponding Rust attributes.
///
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error(inline(never))]
/// #[error("rare error")]
/// struct RareError;
/// ```
///
/// Generated paths use `::core` by default. `import(path)` selects a different
/// root.
///
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error(import(::std))]
/// #[error("standard error")]
/// struct StdError;
/// ```
#[proc_macro_derive(Error, attributes(error))]
pub fn error(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match generate(&input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(error) => error.to_compile_error().into(),
    }
}
