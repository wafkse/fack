//! The structure error type.

use alloc::{format, vec::Vec};
use proc_macro2::TokenStream;
use syn::{FieldsNamed, FieldsUnnamed, Generics, Ident, Type};

use super::common::{FieldRef, Format, ImportRoot, InlineOptions, Transparent};

/// A structure that corresponds to an **error** type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Structure {
    /// The inline behavior for this structure.
    pub inline_opts: Option<InlineOptions>,

    /// The import root for this structure.
    pub root_import: Option<ImportRoot>,

    /// The name of this structure.
    pub name_ident: Ident,

    /// The generic parameters of this structure.
    pub generics: Generics,

    /// The field list of this structure.
    pub field_list: FieldList,

    /// The options for this structure.
    pub options: StructureOptions,
}

/// The options for this structure in terms of its behavior.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[allow(clippy::large_enum_variant)]
pub enum StructureOptions {
    /// The structure is a new, standalone error type.
    Standalone {
        /// The field to be used as the source of the error.
        source_field: Option<FieldRef>,

        /// The format parameters for this structure.
        format_args: Format,
    },

    /// A structure that is transparent over some downstream type.
    Transparent(Transparent),

    /// A structure that is a forwarder to one of its fields.
    Forward {
        /// The field to be used as the source of the error.
        source_field: Option<FieldRef>,

        /// A reference to the field that is being forwarded.
        field_ref: FieldRef,

        /// The type of the forwarded field.
        field_type: Type,

        /// The format parameters for this structure.
        format_args: Format,
    },
}

/// The field list of a structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldList {
    /// A named field list, which is a list of fields that are named.
    Named(Vec<NamedField>),

    /// An unnamed field list, which is a list of fields that are not named.
    Unnamed(Vec<UnnamedField>),

    /// A unit field list, which is equivalent to an absence of fields.
    Unit,
}

impl FieldList {
    /// Convert a target [`syn::Fields`] into a [`FieldList`].
    #[inline]
    pub fn raw(fields: &syn::Fields) -> syn::Result<Self> {
        match fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => {
                let field_list = named
                    .iter()
                    .map(|syn::Field { ident, ty, .. }| {
                        let name = ident
                            .clone()
                            .ok_or_else(|| syn::Error::new_spanned(ident, "expected a named field"))?;

                        let ty = ty.clone();

                        Ok(NamedField { name, ty })
                    })
                    .collect::<syn::Result<Vec<_>>>()?;

                Ok(FieldList::Named(field_list))
            }
            syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                let field_list = unnamed
                    .iter()
                    .map(|syn::Field { ty, .. }| {
                        let ty = ty.clone();

                        Ok(UnnamedField { ty })
                    })
                    .collect::<syn::Result<Vec<_>>>()?;

                Ok(FieldList::Unnamed(field_list))
            }
            syn::Fields::Unit => Ok(FieldList::Unit),
        }
    }

    /// Expect exactly one field in the target [`syn::Fields`].
    pub fn exactly_one(fields: &syn::Fields) -> syn::Result<(FieldRef, Type)> {
        const DEFAULT_FIELD: usize = 0;

        match fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => {
                if named.len() != 1 {
                    return Err(syn::Error::new_spanned(fields, "expected exactly one field"));
                }

                let field = &named[DEFAULT_FIELD];

                let name = field.ident.clone();

                let ty = field.ty.clone();

                Ok((name.map_or(FieldRef::Indexed(DEFAULT_FIELD), FieldRef::Named), ty))
            }
            syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                if unnamed.len() != 1 {
                    return Err(syn::Error::new_spanned(fields, "expected exactly one field"));
                }

                let field = &unnamed[DEFAULT_FIELD];

                let ty = field.ty.clone();

                Ok((FieldRef::Indexed(DEFAULT_FIELD), ty))
            }
            syn::Fields::Unit => Err(syn::Error::new_spanned(fields, "expected at least one field")),
        }
    }

    /// Compute an irrefutable pattern for this field list.
    pub fn pattern(&self) -> syn::Result<TokenStream> {
        match self {
            FieldList::Named(named) => {
                let pattern = named.iter().map(|field| {
                    let name = &field.name;

                    quote::quote! { ref #name }
                });

                Ok(quote::quote! { { #(#pattern),* } })
            }
            FieldList::Unnamed(unnamed) => {
                let pattern = unnamed.iter().enumerate().map(|(index, _)| {
                    let ident = Ident::new(&format!("_{index}"), proc_macro2::Span::call_site());

                    quote::quote! { ref #ident }
                });

                Ok(quote::quote! { ( #(#pattern),* ) })
            }
            FieldList::Unit => Ok(quote::quote! {}),
        }
    }
}

/// A named field that is part of a structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedField {
    /// The name of the field.
    name: Ident,

    /// The type of the field.
    ty: Type,
}

/// An unnamed field that is part of a structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnnamedField {
    /// The type of the field.
    ty: Type,
}
