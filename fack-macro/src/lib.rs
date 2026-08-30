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
/// `transparent(field)` selects one non-optional error field. Extra fields may
/// retain context. Display forwards to the selected field and source chaining
/// forwards through that field's own `Error::source` implementation.
///
/// ```rust
/// # use fack_macro::Error;
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
