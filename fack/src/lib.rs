//! Declarative error handling for Rust with `no_std` support and
//! zero-allocation runtime.
//!
//! This library provides a `#[derive(Error)]` macro that generates complete
//! `Error` and `Display` implementations from declarative attributes.
//!
//! # Features
//!
//! - **`no_std` compatible**: Uses `::core` by default, optional `::std`
//!   support via `#[error(import(::std))]`
//! - **Zero allocation**: No heap allocations in generated error
//!   implementations
//! - **Declarative attributes**: Configure all error behavior through
//!   `#[error(...)]` attributes
//! - **Composable codegen**: Standalone `fack-codegen` library available for
//!   custom tooling
//! - **Span preservation**: Maintains accurate source spans for IDE hints and
//!   LSP integration
//!
//! # Quick Start
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("file not found: {path}")]
//! struct FileError {
//!     path: String,
//! }
//! ```
//!
//! # Attribute Reference
//!
//! ## Format String: `#[error("...")]`
//!
//! A `Display` implementation is generated for your error type. The format
//! string supports field interpolation using standard Rust formatting syntax.
//!
//! **Named fields**: Reference directly by name in the format string:
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("file not found: {path}")]
//! struct FileError {
//!     path: String,
//! }
//! ```
//!
//! **Tuple struct fields**: Use `_N` syntax (underscore prefix for numeric
//! indices):
//!
//! ```rust
//! use fack::prelude::*;
//!
//! #[derive(Error, Debug)]
//! #[error("invalid value: {_0}")]
//! struct ValueError(i32);
//! ```
//!
//! **Multiple fields**:
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("range error: {value} not in {min}..{max}")]
//! struct RangeError {
//!     value: i32,
//!     min: i32,
//!     max: i32,
//! }
//! ```
//!
//! **Format specifiers**:
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("parse error: {input:?}")]
//! struct ParseError {
//!     input: String,
//! }
//! ```
//!
//! All format placeholders and specifiers from `std::fmt` are supported.
//!
//! ## Source Declaration: `#[error(source(field))]`
//!
//! The `Error::source()` method is implemented to return the specified field.
//! This enables error chain traversal for debugging and logging. Any type
//! implementing `std::error::Error` can be a source.
//!
//! **Named fields**:
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("network request failed")]
//! #[error(source(inner))]
//! struct NetworkError {
//!     inner: std::io::Error,
//!     url: String,
//! }
//! ```
//!
//! **Tuple fields** (by index):
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("wrapped error")]
//! #[error(source(cause))]
//! struct Wrapper {
//!     cause: std::io::Error,
//!     context: String,
//! }
//! ```
//!
//! The generated `source()` method returns `Option<&(dyn std::error::Error +
//! 'static)>`.
//!
//! ## Transparent Forwarding: `#[error(transparent(field))]`
//!
//! Forwards both `Display` and `Error::source()` directly to the specified
//! field without adding additional formatting. The error type becomes a
//! transparent wrapper.
//!
//! **Use case 1: Enum catch-all variant**:
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! enum MyError {
//!     #[error("known error type")]
//!     Known(String),
//!
//!     #[error(transparent(0))]
//!     Io(std::io::Error),
//! }
//! ```
//!
//! **Use case 2: Opaque public API**:
//! ```rust
//! # use fack::prelude::*;
//! // Public error type that hides implementation details
//! #[derive(Error, Debug)]
//! #[error(transparent(inner))]
//! pub struct PublicError {
//!     inner: InternalError,
//! }
//!
//! // Private, can change without breaking public API
//! #[derive(Error, Debug)]
//! #[error("internal error")]
//! struct InternalError {
//!     details: String,
//! }
//! ```
//!
//! The specified field must implement `std::error::Error`.
//!
//! ## Automatic Conversion: `#[error(from)]`
//!
//! Generates a `From<T>` implementation for automatic error conversion via the
//! `?` operator. The type must have exactly one field.
//!
//! **Enum variants**:
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! enum AppError {
//!     #[error("io error occurred")]
//!     #[error(from)]
//!     Io(std::io::Error),
//!
//!     #[error("parse error occurred")]
//!     #[error(from)]
//!     Parse(std::num::ParseIntError),
//! }
//! ```
//!
//! This generates:
//! ```rust,ignore
//! impl From<std::io::Error> for AppError {
//!     fn from(source: std::io::Error) -> Self {
//!         AppError::Io(source)
//!     }
//! }
//! // ... and similar for ParseIntError
//! ```
//!
//! **Structs**:
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("io operation failed: {_0}")]
//! #[error(from)]
//! struct IoError(std::io::Error);
//! ```
//!
//! **Important**: `#[error(from)]` automatically implies the field is also the
//! error source, so you don't need to specify `#[error(source(...))]`
//! separately.
//!
//! ## Inline Control: `#[error(inline(strategy))]`
//!
//! Controls inlining behavior of generated trait implementations. This affects
//! all generated methods: `fmt` (Display), `source` (Error), and `from` (if
//! applicable).
//!
//! **Strategies**:
//!
//! - **`neutral`** (default): Emits `#[inline]`, which suggests inlining but
//!   lets the compiler decide. Appropriate for most error types.
//!
//! - **`always`**: Emits `#[inline(always)]`, forcing aggressive inlining. Use
//!   for performance-critical error paths where function call overhead matters:
//!
//!   ```rust
//!   # use fack::prelude::*;
//!   #[derive(Error, Debug)]
//!   #[error(inline(always))]
//!   #[error("hot path error: {code}")]
//!   struct FastError {
//!       code: u32,
//!   }
//!   ```
//!
//! - **`never`**: Emits `#[inline(never)]`, preventing inlining. Use to reduce
//!   code size for rarely constructed errors or to maintain consistent stack
//!   traces:
//!
//!   ```rust
//!   # use fack::prelude::*;
//!   #[derive(Error, Debug)]
//!   #[error(inline(never))]
//!   #[error("rare error condition")]
//!   struct RareError {
//!       details: String,
//!   }
//!   ```
//!
//! When applied at the enum level, the strategy affects all variants.
//!
//! ## Import Root: `#[error(import(path))]`
//!
//! Overrides the default `::core` import path used in generated code. By
//! default, all generated code uses `::core` for `no_std` compatibility.
//!
//! **Using `std` explicitly**:
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error(import(::std))]
//! #[error("standard library error")]
//! struct StdError {
//!     message: String,
//! }
//! ```
//!
//! This generates:
//! ```rust,ignore
//! impl ::std::error::Error for StdError { /* ... */ }
//! impl ::std::fmt::Display for StdError { /* ... */ }
//! ```
//!
//! **Custom preludes**:
//! ```rust,ignore
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error(import(crate::prelude))]
//! #[error("custom prelude error")]
//! struct CustomError {
//!     msg: String,
//! }
//! ```
//!
//! **Use cases**:
//! - Explicitly targeting `std` in std-only crates
//! - Working with custom error trait definitions
//! - Integrating with frameworks providing error handling primitives
//!
//! # Enum Support
//!
//! Enums support per-variant attribute configuration:
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! enum FileError {
//!     #[error("file not found: {path}")]
//!     NotFound { path: String },
//!
//!     #[error("permission denied: {_0}")]
//!     #[error(source(0))]
//!     PermissionDenied(std::io::Error),
//!
//!     #[error(transparent(0))]
//!     Io(std::io::Error),
//! }
//! ```
//!
//! Type-level attributes apply to all variants:
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error(inline(always))]  // Applies to all variants
//! #[error(import(::std))]   // Applies to all variants
//! enum OptimizedError {
//!     #[error("first variant")]
//!     First,
//!     #[error("second variant")]
//!     Second,
//! }
//! ```
//!
//! # Type Support
//!
//! **Supported types**:
//! - Structs with named fields
//! - Structs with unnamed fields (tuple structs)
//! - Unit structs
//! - Enums with any variant style
//!
//! **Unsupported types**:
//! - Union types (unions cannot implement `Error`)
//!
//! # Generated Code Guarantees
//!
//! All generated implementations:
//! - Include `#[automatically_derived]` for tool recognition
//! - Properly destructure fields in pattern matches
//! - Correctly forward lifetimes and generic parameters
//! - Contain no unsafe code
//! - Require no heap allocations at runtime
//! - Are `no_std` compatible (use `::core` by default)
#![no_std]
#![forbid(unsafe_code, missing_docs, rustdoc::all, clippy::all, clippy::pedantic)]

pub mod prelude {
    //! Re-exports the most commonly used traits and types.

    pub use fack_core::prelude::*;

    pub use fack_macro::Error;
}
