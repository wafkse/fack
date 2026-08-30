//! Validated error semantics carried into code generation.
//!
//! This subsystem owns proof-bearing display, source, conversion, and variant
//! states. Its values are constructed only by semantic validation and are the
//! complete input accepted by expansion.

use alloc::{boxed::Box, vec::Vec};

use syn::{Generics, Ident, Path};

use crate::input::{Field, Fields, ImportRoot, InlinePolicy};

mod format;
mod source;
mod validate;

pub use format::{BindingRequirement, FormatTrait, Resolved};
pub use source::{Source, TransparentSource};

/// A validated error declaration ready for expansion.
#[derive(Clone, Debug)]
pub enum Target {
    /// A validated structure.
    Struct(Box<Structure>),

    /// A validated enumeration.
    Enum(Enumeration),
}

/// Shared validated identity and generation options for an error type.
#[derive(Clone, Debug)]
// NOTE(invariant): All generation options and type identity belong to one validated derive target.
pub struct Header {
    /// Validated generated inline policy.
    inline: InlinePolicy,

    /// Validated generated root import.
    import: ImportRoot,

    /// Validated error type identifier.
    name: Ident,

    /// Generic parameters of the generated error type.
    generics: Generics,
}

impl Header {
    /// Construct a validated error type header.
    #[must_use]
    #[inline]
    pub const fn new(inline: InlinePolicy, import: ImportRoot, name: Ident, generics: Generics) -> Self {
        Self {
            inline,
            import,
            name,
            generics,
        }
    }

    /// Consume the header into generation options and type identity.
    #[inline]
    pub fn parts(self) -> (InlinePolicy, ImportRoot, Ident, Generics) {
        let Self {
            inline,
            import,
            name,
            generics,
        } = self;

        (inline, import, name, generics)
    }
}

/// A validated structure error.
#[derive(Clone, Debug)]
// NOTE(invariant): Every resolved field in display, source, or conversion state was produced from this structure field set.
pub struct Structure {
    /// Validated type identity and generation options.
    header: Header,

    /// Validated structural fields.
    fields: Fields,

    /// Validated display behavior.
    display: Display,

    /// Validated source-chain behavior.
    source: ErrorSource,

    /// Optional validated automatic conversion.
    conversion: Option<Conversion>,
}

impl Structure {
    /// Construct validated structure semantics.
    #[must_use]
    #[inline]
    pub const fn new(header: Header, fields: Fields, display: Display, source: ErrorSource, conversion: Option<Conversion>) -> Self {
        Self {
            header,
            fields,
            display,
            source,
            conversion,
        }
    }

    /// Consume the structure into its validated semantic parts.
    #[inline]
    pub fn parts(self) -> (Header, Fields, Display, ErrorSource, Option<Conversion>) {
        let Self {
            header,
            fields,
            display,
            source,
            conversion,
        } = self;

        (header, fields, display, source, conversion)
    }
}

/// A validated enumeration error.
#[derive(Clone, Debug)]
// NOTE(invariant): Every stored variant is validated under the same enumeration header.
pub struct Enumeration {
    /// Validated type identity and generation options.
    header: Header,

    /// Validated error variants.
    variants: Vec<Variant>,
}

impl Enumeration {
    /// Construct validated enumeration semantics.
    #[inline]
    #[must_use]
    pub const fn new(header: Header, variants: Vec<Variant>) -> Self {
        Self { header, variants }
    }

    /// Consume the enumeration into its validated semantic parts.
    #[inline]
    pub fn parts(self) -> (Header, Vec<Variant>) {
        let Self { header, variants } = self;

        (header, variants)
    }
}

/// A validated enumeration variant.
#[derive(Clone, Debug)]
// NOTE(invariant): Every resolved field in display, source, or conversion state was produced from this variant field set.
pub struct Variant {
    /// Validated source variant identifier.
    name: Ident,

    /// Validated variant fields.
    fields: Fields,

    /// Validated display behavior.
    display: Display,

    /// Validated source-chain behavior.
    source: ErrorSource,

    /// Optional validated automatic conversion.
    conversion: Option<Conversion>,
}

impl Variant {
    /// Construct validated variant semantics.
    #[must_use]
    #[inline]
    pub const fn new(name: Ident, fields: Fields, display: Display, source: ErrorSource, conversion: Option<Conversion>) -> Self {
        Self {
            name,
            fields,
            display,
            source,
            conversion,
        }
    }

    /// Return the variant name.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &Ident {
        let &Self { ref name, .. } = self;

        name
    }

    /// Return the variant fields.
    #[inline]
    #[must_use]
    pub const fn fields(&self) -> &Fields {
        let &Self { ref fields, .. } = self;

        fields
    }

    /// Return display semantics.
    #[inline]
    #[must_use]
    pub const fn display(&self) -> &Display {
        let &Self { ref display, .. } = self;

        display
    }

    /// Return source semantics.
    #[inline]
    #[must_use]
    pub const fn source(&self) -> &ErrorSource {
        let &Self { ref source, .. } = self;

        source
    }

    /// Return conversion semantics.
    #[inline]
    #[must_use]
    pub const fn conversion(&self) -> Option<&Conversion> {
        let &Self { ref conversion, .. } = self;

        conversion.as_ref()
    }
}

/// Validated display behavior.
#[derive(Clone, Debug)]
pub enum Display {
    /// A resolved Rust format string.
    Format(Resolved),

    /// Transparent forwarding to the selected field.
    Transparent(Field),

    /// A custom formatter function.
    Custom(Path),
}

/// Validated source behavior.
#[derive(Clone, Debug)]
pub enum ErrorSource {
    /// The error has no source.
    None,

    /// The error exposes one ordinary source field.
    Field(Source),

    /// The error forwards through the selected transparent field.
    Transparent(TransparentSource),
}

/// One validated automatic conversion.
#[derive(Clone, Debug)]
// NOTE(invariant): The stored field is the sole validated destination of the automatic conversion.
pub struct Conversion {
    /// Resolved field used by the conversion.
    field: Field,
}

impl Conversion {
    /// Construct a conversion from its resolved field and type.
    #[inline]
    #[must_use]
    pub const fn new(field: Field) -> Self {
        Self { field }
    }

    /// Return the converted field.
    #[inline]
    #[must_use]
    pub const fn field(&self) -> &Field {
        let &Self { ref field } = self;

        field
    }
}
