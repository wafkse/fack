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
//! `#[error(transparent(field))]` selects one non-optional error field. Extra
//! fields may retain context. Display forwards to the selected field and source
//! chaining forwards through that field's own `Error::source` implementation.
//!
//! ```rust
//! # use fack::prelude::*;
//! #[derive(Error, Debug)]
//! #[error(transparent(inner))]
//! struct Wrapper {
//!     context: u8,
//!     inner: std::io::Error,
//! }
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

pub mod prelude {
    //! The `fack` prelude.
    //!
    //! Re-exports the standard error trait and the `fack` derive macro for
    //! applications that prefer one common import.

    pub use core::error::Error;

    pub use fack_macro::Error;
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::{boxed::Box, string::ToString};
    use core::fmt::{Formatter, Result as FmtResult};

    use super::prelude::*;

    #[derive(Debug, Error)]
    #[error("inner failure")]
    struct Inner;

    #[derive(Debug, Error)]
    #[error("outer failure")]
    #[error(source(inner))]
    struct StructSource {
        pub inner: Inner,
    }

    #[derive(Debug, Error)]
    enum EnumSource {
        #[error("named failure")]
        #[error(source(inner))]
        Named { inner: Inner, _code: u8 },

        #[error("tuple failure")]
        #[error(source(0))]
        Tuple(Inner, ()),
    }

    #[test]
    fn structure_source_is_selected_field() {
        let error = StructSource { inner: Inner };
        let source = error.source().expect("declared source must exist");

        assert_eq!(source.to_string(), "inner failure");
    }

    #[test]
    fn enum_sources_are_selected_bindings() {
        let named = EnumSource::Named { inner: Inner, _code: 7 };
        let tuple = EnumSource::Tuple(Inner, ());

        let named_source = named.source().expect("named source must exist");
        let tuple_source = tuple.source().expect("tuple source must exist");

        assert_eq!(named_source.to_string(), "inner failure");
        assert_eq!(tuple_source.to_string(), "inner failure");
    }

    #[derive(Debug, Error)]
    #[error("from struct")]
    #[error(from)]
    struct StructFrom(pub Inner);

    #[derive(Debug, Error)]
    enum EnumFrom {
        #[error("from enum")]
        #[error(from)]
        Inner(Inner),
    }

    #[test]
    fn from_implies_the_converted_field_is_the_source() {
        let structure = StructFrom::from(Inner);
        let enumeration = EnumFrom::from(Inner);

        assert_eq!(structure.source().expect("from source must exist").to_string(), "inner failure");
        assert_eq!(enumeration.source().expect("from source must exist").to_string(), "inner failure");
    }

    #[derive(Debug, Error)]
    #[error(transparent(0))]
    struct Transparent(pub Inner);

    #[test]
    fn transparent_forwards_display_and_inner_source() {
        let error = Transparent(Inner);

        assert_eq!(error.to_string(), "inner failure");
        assert!(error.source().is_none());
    }

    #[derive(Debug, Error)]
    #[error("transparent root failure")]
    struct TransparentRoot;

    #[derive(Debug, Error)]
    #[error("transparent inner failure")]
    #[error(source(root))]
    struct TransparentInner {
        /// Root cause forwarded by transparent wrappers.
        pub root: TransparentRoot,
    }

    /// Context value deliberately lacking `Display` and `Error`.
    #[derive(Debug)]
    struct TransparentContextValue;

    #[derive(Debug, Error)]
    #[error(transparent(inner))]
    struct TransparentContext {
        /// Context retained without participating in transparent behavior.
        pub _context: TransparentContextValue,

        /// Selected error providing display and source-chain behavior.
        pub inner: TransparentInner,
    }

    #[derive(Debug, Error)]
    enum TransparentEnum {
        /// Transparent variant retaining unrelated context.
        #[error(transparent(inner))]
        Context {
            /// Context retained without participating in transparent behavior.
            _context: TransparentContextValue,

            /// Selected error providing display and source-chain behavior.
            inner: TransparentInner,
        },
    }

    #[test]
    fn transparent_selected_field_allows_additional_context() {
        let structure = TransparentContext {
            _context: TransparentContextValue,
            inner: TransparentInner { root: TransparentRoot },
        };
        let enumeration = TransparentEnum::Context {
            _context: TransparentContextValue,
            inner: TransparentInner { root: TransparentRoot },
        };

        assert_eq!(structure.to_string(), "transparent inner failure");
        assert_eq!(enumeration.to_string(), "transparent inner failure");
        assert_eq!(
            structure.source().expect("transparent structure source").to_string(),
            "transparent root failure"
        );
        assert_eq!(
            enumeration.source().expect("transparent enum source").to_string(),
            "transparent root failure"
        );
    }

    #[derive(Debug, Error)]
    #[error("tuple {0} debug {1:?} hex {2:#x}")]
    struct TupleFormat(pub u8, pub &'static str, pub u16);

    #[derive(Debug, Error)]
    #[error("named {name} debug {value:?} hex {code:#x}")]
    struct NamedFormat {
        pub name: &'static str,
        pub value: Option<u8>,
        pub code: u16,
    }

    #[test]
    fn format_strings_capture_tuple_and_named_fields() {
        let tuple = TupleFormat(7, "text", 0x2a);
        let named = NamedFormat {
            name: "item",
            value: Some(3),
            code: 0x2a,
        };

        assert_eq!(tuple.to_string(), "tuple 7 debug \"text\" hex 0x2a");
        assert_eq!(named.to_string(), "named item debug Some(3) hex 0x2a");
    }

    #[derive(Debug, Error)]
    #[error("escaped {{field}} and {}", value.abs())]
    struct ExpressionFormat {
        pub value: i8,
    }

    #[test]
    fn format_strings_preserve_explicit_arguments_and_escaped_braces() {
        let error = ExpressionFormat { value: -7 };

        assert_eq!(error.to_string(), "escaped {field} and 7");
    }

    #[derive(Debug, Error)]
    #[error("generic {0}")]
    struct GenericDisplay<T>(pub T);

    #[derive(Debug, Error)]
    #[error("generic debug {0:?}")]
    struct GenericDebug<T>(pub T);

    #[test]
    fn format_captures_add_only_required_format_bounds() {
        let display = GenericDisplay(17_u8);
        let debug = GenericDebug(Some(4_u8));

        assert_eq!(display.to_string(), "generic 17");
        assert_eq!(debug.to_string(), "generic debug Some(4)");
    }

    #[derive(Debug, Error)]
    #[error("boxed source")]
    #[error(source(inner))]
    struct BoxedSource {
        pub inner: Box<Inner>,
    }

    #[derive(Debug, Error)]
    #[error("optional source")]
    #[error(source(inner))]
    struct OptionalSource {
        pub inner: Option<Inner>,
    }

    #[derive(Debug, Error)]
    #[error("optional boxed source")]
    #[error(source(inner))]
    struct OptionalBoxedSource {
        pub inner: Option<Box<Inner>>,
    }

    #[derive(Debug, Error)]
    #[error("dynamic source")]
    #[error(source(inner))]
    struct DynamicSource {
        pub inner: Box<dyn Error + Send + Sync>,
    }

    #[test]
    fn common_source_container_shapes_are_supported() {
        let boxed = BoxedSource { inner: Box::new(Inner) };
        let optional = OptionalSource { inner: Some(Inner) };
        let absent = OptionalSource { inner: None };
        let optional_boxed = OptionalBoxedSource {
            inner: Some(Box::new(Inner)),
        };
        let dynamic = DynamicSource { inner: Box::new(Inner) };

        assert_eq!(boxed.source().expect("boxed source").to_string(), "inner failure");
        assert_eq!(optional.source().expect("optional source").to_string(), "inner failure");
        assert!(absent.source().is_none());
        assert_eq!(optional_boxed.source().expect("optional boxed source").to_string(), "inner failure");
        assert_eq!(dynamic.source().expect("dynamic source").to_string(), "inner failure");
    }

    #[derive(Debug, Error)]
    #[error("named argument {rendered}", rendered = value.abs())]
    struct NamedArgument {
        pub value: i8,
    }

    #[test]
    fn explicit_named_format_arguments_take_precedence_over_field_capture() {
        let error = NamedArgument { value: -9 };

        assert_eq!(error.to_string(), "named argument 9");
    }

    fn custom_format(error: &CustomDisplay, formatter: &mut Formatter<'_>) -> FmtResult {
        let &CustomDisplay { value } = error;

        write!(formatter, "custom {value}")
    }

    #[derive(Debug, Error)]
    #[error(display(custom_format))]
    struct CustomDisplay {
        pub value: u8,
    }

    #[test]
    fn custom_display_formatter_is_an_escape_hatch() {
        assert_eq!(CustomDisplay { value: 5 }.to_string(), "custom 5");
    }

    #[derive(Debug)]
    struct DebugOnly;

    #[derive(Debug, Error)]
    #[error("selected {selected}")]
    struct SelectiveFormat<T, U> {
        pub selected: T,
        pub _unused: U,
    }

    #[test]
    fn unused_fields_do_not_acquire_format_bounds() {
        let error = SelectiveFormat {
            selected: 3_u8,
            _unused: DebugOnly,
        };

        assert_eq!(error.to_string(), "selected 3");
    }

    #[derive(Debug, Error)]
    #[error("named {value:>width$}")]
    struct NamedWidth {
        pub value: u8,
        pub width: usize,
    }

    #[derive(Debug, Error)]
    #[error("tuple {0:>1$}")]
    struct TupleWidth(pub u8, pub usize);

    #[test]
    fn dynamic_width_fields_are_resolved_and_bound() {
        assert_eq!(NamedWidth { value: 7, width: 3 }.to_string(), "named   7");
        assert_eq!(TupleWidth(7, 3).to_string(), "tuple   7");
    }

    #[derive(Debug, Error)]
    #[error("generic from")]
    #[error(from)]
    struct GenericFrom<E>(pub E);

    #[test]
    fn generic_from_adds_the_source_bound_to_only_the_error_impl() {
        let error = GenericFrom::from(Inner);

        assert_eq!(error.source().expect("generic source").to_string(), "inner failure");
    }

    fn format_custom_enum(error: &CustomEnum, formatter: &mut Formatter<'_>) -> FmtResult {
        match error {
            &CustomEnum::Value(value) => write!(formatter, "enum custom {value}"),
        }
    }

    #[derive(Debug, Error)]
    enum CustomEnum {
        #[error(display(format_custom_enum))]
        Value(u8),
    }

    #[test]
    fn custom_display_formatter_works_for_enum_variants() {
        assert_eq!(CustomEnum::Value(8).to_string(), "enum custom 8");
    }

    /// Invalid declarations fail with diagnostics through the exported derive
    /// macro.
    #[test]
    fn invalid_error_declarations_fail_with_fack_diagnostics() {
        let cases = trybuild::TestCases::new();

        cases.compile_fail("tests/ui/*.rs");
    }
}
