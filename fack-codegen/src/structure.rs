//! Parsed structure declarations before semantic validation.

use syn::{Generics, Ident};

use crate::{
    field::Fields,
    syntax::{Config, Declaration},
};

/// A parsed structure error declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Structure {
    /// Parsed container-wide generation options.
    config: Config,

    /// Source structure identifier.
    name: Ident,

    /// Source generic parameters.
    generics: Generics,

    /// Parsed structural fields.
    fields: Fields,

    /// Parsed error behavior declarations.
    declaration: Declaration,
}

impl Structure {
    /// Construct a parsed structure declaration.
    #[inline]
    #[must_use]
    pub const fn new(config: Config, name: Ident, generics: Generics, fields: Fields, declaration: Declaration) -> Self {
        Self {
            config,
            name,
            generics,
            fields,
            declaration,
        }
    }

    /// Consume the structure into its parsed components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Config, Ident, Generics, Fields, Declaration) {
        let Self {
            config,
            name,
            generics,
            fields,
            declaration,
        } = self;
        (config, name, generics, fields, declaration)
    }
}
