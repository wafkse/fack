use proc_macro::TokenStream;

use fack_codegen::{Target, expand::Expand};

/// Derive the `Error` trait for a struct.
///
/// The `#[derive(Error)]` macro provides a declarative way to implement
/// both the [`Error`] and [`Display`] traits for your custom error types.
/// It is controlled by `#[error(...)]` *type-level attributes* that specify
/// how the error should behave.
///
/// # General form
///
/// ```ignore
/// #[derive(Error, Debug)]
/// #[error("a formatted message {arg}", arg)]
/// // #[error(source(...))]
/// // #[error(transparent(...))]
/// // #[error(from)]
/// struct MyError { /* fields */ }
/// ```
///
/// The attribute syntax follows the form:
///
/// ```text
/// #[error(<parameter>)]
/// ```
///
/// Multiple `#[error(...)]` attributes can be applied to the same type,
/// and are classified according to their *parameter kind*.
///
///
/// # Attribute kinds
///
/// The following attribute kinds are supported:
///
/// ## `#[error("...")]` - format string
///
/// Provides a [`Display`] implementation for the error type, using
/// Rust formatting syntax. Additional expressions may be listed as
/// comma-separated arguments.
///
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error("failed to read file: {path}")]
/// struct ReadError {
///     path: String,
/// }
/// ```
///
/// Expands to:
///
/// ```ignore
/// impl std::fmt::Display for ReadError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "failed to read file: {}", self.path)
///     }
/// }
/// ```
///
/// ## `#[error(source(<field>))]` - source field
///
/// Declares which field represents the underlying cause of the error.
/// This field will be returned from `Error::source`.
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
/// Produces:
/// ```ignore
/// impl std::error::Error for NetworkError {
///     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
///         Some(&self.io)
///     }
/// }
/// ```
///
/// ## `#[error(transparent(<field>))]` - transparent wrapper
///
/// Makes the error type a transparent wrapper around one of its fields.
/// Both [`Display`] and [`Error`] are forwarded directly to that field.
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error(transparent(0))]
/// struct Wrapper(std::io::Error);
/// ```
///
/// This makes `Wrapper` behave exactly like its inner error for
/// formatting and chaining purposes.
///
///
/// ## `#[error(from)]` conversion
///
/// Requests that a `From<T>` implementation be generated for a specified
/// field type, allowing automatic conversion into the error type.
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error("parse failed")]
/// #[error(from)]
/// struct ParseError {
///     inner: std::num::ParseIntError,
/// }
/// ```
///
/// Allows:
/// ```ignore
/// let err: ParseError = "abc".parse::<u32>().unwrap_err().into();
/// ```
///
///
/// # Enums
///
/// Enums may use the same attribute kinds at the *variant* level.
/// Each variant can provide its own formatting, source, or transparency.
/// ```
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// enum MyError {
///     #[error("invalid input: {_0}")]
///     InvalidInput(String),
///
///     #[error(transparent(inner))]
///     Io {
///         inner: std::io::Error,
///     },
///
///     #[error("other error")]
///     Other,
/// }
/// ```
///
///
/// # Inline options
///
/// Inline behavior for generated methods can be controlled using:
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error(inline(always))]
/// #[error("{msg}")]
/// struct FastError {
///     msg: &'static str,
/// }
/// ```
///
/// Valid values are:
/// - `neutral` (default) → `#[inline]`
/// - `always` → `#[inline(always)]`
/// - `never` → `#[inline(never)]`
///
/// When omitted, no explicit `#[inline(...)]` attribute is emitted.
///
///
/// # Root import
///
/// All generated code defaults to using `::core`. This can be overridden:
/// ```rust
/// # use fack_macro::Error;
/// #[derive(Error, Debug)]
/// #[error("{msg}")]
/// #[error(import(::std))]
/// struct StdError {
///     msg: &'static str,
/// }
/// ```
///
/// This allows compatibility with `std::error::Error` where desired.
///
/// [`Error`]: error
/// [`Display`]: core::fmt::Display
#[proc_macro_derive(Error, attributes(error))]
pub fn error(input: TokenStream) -> TokenStream {
    let input = &syn::parse_macro_input!(input as syn::DeriveInput);

    match Target::input(input).map(Expand::target) {
        Ok(Ok(target_stream)) => TokenStream::from(target_stream),
        Err(target_error) | Ok(Err(target_error)) => target_error.to_compile_error().into(),
    }
}
