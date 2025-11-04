//! The enumeration error type.

use alloc::vec::Vec;

use syn::{Generics, Ident, Type};

use super::{
    common::{FieldRef, Format, ImportRoot, InlineOptions, Transparent},
    structure::FieldList,
};

/// An enumeration that corresponds to an **error** type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Enumeration {
    /// The inline behavior for this enumeration.
    pub inline_opts: Option<InlineOptions>,

    /// The import root for this enumeration.
    pub root_import: Option<ImportRoot>,

    /// The identifier of this enumeration.
    pub name_ident: Ident,

    /// The generic parameters of this enumeration.
    pub generics: Generics,

    /// The variant list of this enumeration.
    pub variant_list: Vec<Variant>,
}

/// A single variant of an enumeration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    /// A struct-like variant that is composed of a list of fields.
    Struct {
        /// The formatter setting for this variant.
        format: Format,

        /// The field to be used as the source of the error.
        source_field: Option<FieldRef>,

        /// The name of the current variant.
        variant_name: Ident,

        /// The field list of this struct-like variant.
        field_list: FieldList,
    },

    /// An unit variant that has no fields.
    Unit {
        /// The formatter setting for this variant.
        format: Format,

        /// The name of the current variant.
        variant_name: Ident,
    },

    /// A transparent variant that is transparent over some downstream type.s
    Transparent {
        /// The transparence setting for this variant.
        transparent: Transparent,

        /// The name of the current variant.
        variant_name: Ident,

        /// The field list of this struct-like variant.
        field_list: FieldList,
    },

    /// A forward variant that forwards to one of its fields.
    Forward {
        /// The formatter setting for this variant.
        format: Format,

        /// The name of the current variant.
        variant_name: Ident,

        /// The forwarded field.
        field_ref: FieldRef,

        /// The type of the forwarded field.
        field_type: Type,
    },
}
