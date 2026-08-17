//! Parsed enumeration declarations before semantic validation.

use alloc::vec::Vec;
use syn::{Generics, Ident};

use crate::{
    field::Fields,
    syntax::{Config, Declaration},
};

/// A parsed enumeration error declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Enumeration {
    /// Parsed container-wide generation options.
    config: Config,

    /// Source enumeration identifier.
    name: Ident,

    /// Source generic parameters.
    generics: Generics,

    /// Parsed error variants.
    variants: Vec<Variant>,
}

impl Enumeration {
    /// Construct a parsed enumeration declaration.
    #[inline]
    #[must_use]
    pub const fn new(config: Config, name: Ident, generics: Generics, variants: Vec<Variant>) -> Self {
        Self {
            config,
            name,
            generics,
            variants,
        }
    }

    /// Consume the enumeration into its parsed components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Config, Ident, Generics, Vec<Variant>) {
        let Self {
            config,
            name,
            generics,
            variants,
        } = self;

        (config, name, generics, variants)
    }
}

/// A parsed error enum variant declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variant {
    /// Source variant identifier.
    name: Ident,

    /// Parsed variant fields.
    fields: Fields,

    /// Parsed error behavior declarations.
    declaration: Declaration,
}

impl Variant {
    /// Construct a parsed enum variant declaration.
    #[inline]
    #[must_use]
    pub const fn new(name: Ident, fields: Fields, declaration: Declaration) -> Self {
        Self { name, fields, declaration }
    }

    /// Consume the variant into its parsed components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Ident, Fields, Declaration) {
        let Self { name, fields, declaration } = self;
        (name, fields, declaration)
    }
}
