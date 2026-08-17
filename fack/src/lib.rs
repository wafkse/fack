//! Declarative error derivation with `no_std` support and zero allocation in
//! generated runtime code.
//!
//! `fack` generates `Display`, `Error`, and optional `From` implementations
//! from `#[error(...)]` declarations.
//!
//! # Quick start
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("file not found {path}")]
//! struct FileError {
//!     path: String,
//! }
//! ```
//!
//! # Formatting
//!
//! Named and tuple fields can be captured directly. Explicit Rust format
//! arguments are also supported.
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("invalid input {input:?}")]
//! struct InputError {
//!     input: String,
//! }
//!
//! #[derive(Error, Debug)]
//! #[error("invalid value {0}")]
//! struct ValueError(i32);
//! ```
//!
//! A custom formatter can be selected when a format string is not the right
//! abstraction.
//!
//! ```rust
//! # use fack::prelude::*;
//! fn render(error: &RenderedError, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//!     let RenderedError { code } = error;
//!     write!(f, "rendered {code}")
//! }
//!
//! #[derive(Error, Debug)]
//! #[error(display(render))]
//! struct RenderedError {
//!     code: u8,
//! }
//! ```
//!
//! # Sources
//!
//! `#[error(source(field))]` exposes the selected field as the ordinary source.
//! Direct, boxed, optional, and optional boxed source fields are supported.
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error("request failed")]
//! #[error(source(io))]
//! struct RequestError {
//!     io: std::io::Error,
//!     url: String,
//! }
//! ```
//!
//! `#[error(transparent(field))]` requires exactly one non-optional field.
//! Display forwards to that field and source chaining forwards through the
//! field's own `Error::source` implementation.
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error(transparent(0))]
//! struct Wrapper(std::io::Error);
//! ```
//!
//! # Conversion
//!
//! `#[error(from)]` requires exactly one field. It generates `From<T>` and
//! makes that field the ordinary error source.
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! enum AppError {
//!     #[error("io error")]
//!     #[error(from)]
//!     Io(std::io::Error),
//!
//!     #[error("parse error")]
//!     #[error(from)]
//!     Parse(std::num::ParseIntError),
//! }
//! ```
//!
//! # Generation options
//!
//! Without an inline declaration, generated methods receive no explicit inline
//! attribute. `#[error(inline)]` and `#[error(inline(neutral))]` emit ordinary
//! `#[inline]`. The `always` and `never` strategies emit their corresponding
//! Rust attributes.
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error(inline(never))]
//! #[error("rare error")]
//! struct RareError;
//! ```
//!
//! Generated paths use `::core` by default. `#[error(import(path))]` selects a
//! different root.
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error(import(::std))]
//! #[error("standard error")]
//! struct StdError;
//! ```
//!
//! Enum-wide generation options belong on the enum. Display, source,
//! transparency, and conversion declarations belong on variants.
//!
//! # Generated code guarantees
//!
//! Generated implementations contain no unsafe code and allocate nothing at
//! runtime. Formatting and source bounds are added only where the generated
//! implementation requires them.
#![no_std]
#![forbid(unsafe_code, missing_docs, rustdoc::all, clippy::all, clippy::pedantic)]

pub mod prelude {
    //! Commonly used `fack` exports.

    pub use fack_core::prelude::*;

    pub use fack_macro::Error;
}
