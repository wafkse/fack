//! Automatic `From` implementation expansion.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Generics, Ident};

use crate::{
    field::Fields,
    semantics::{Conversion, Variant},
};

use super::{Context, Expand};

/// One generated `From` implementation request.
pub struct ConversionImpl<'target> {
    /// Generated error type identifier.
    enum_name: &'target Ident,

    /// Generated error type generic parameters.
    generics: &'target Generics,

    /// Fields used to construct the converted value.
    fields: &'target Fields,

    /// Optional enum variant receiving the converted value.
    variant: Option<&'target Ident>,

    /// Validated conversion semantics.
    conversion: &'target Conversion,
}

impl<'target> ConversionImpl<'target> {
    /// Construct expansion state for a structure conversion.
    pub const fn structure(
        name: &'target Ident,
        generics: &'target Generics,
        fields: &'target Fields,
        conversion: &'target Conversion,
    ) -> Self {
        Self {
            enum_name: name,
            generics,
            fields,
            variant: None,
            conversion,
        }
    }

    /// Construct expansion state for an enum variant conversion.
    pub const fn variant(
        name: &'target Ident,
        generics: &'target Generics,
        variant: &'target Variant,
        conversion: &'target Conversion,
    ) -> Self {
        Self {
            enum_name: name,
            generics,
            fields: variant.fields(),
            variant: Some(variant.name()),
            conversion,
        }
    }
}

impl Expand for ConversionImpl<'_> {
    /// Shared generation options used by the generated `From` implementation.
    type Context = Context;

    /// Generate one `From` implementation from validated conversion semantics.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self {
            enum_name,
            generics,
            fields,
            variant,
            conversion,
        } = self;
        let inline = context.inline();
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let field_type = conversion.field_type();
        let construct = match (variant, fields.name(conversion.field())) {
            (None, Some(field)) => quote! { Self { #field: value } },
            (None, None) => quote! { Self(value) },
            (Some(variant), Some(field)) => quote! { Self::#variant { #field: value } },
            (Some(variant), None) => quote! { Self::#variant(value) },
        };

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics From<#field_type> for #enum_name #ty_generics #where_clause {
                #inline
                fn from(value: #field_type) -> Self {
                    #construct
                }
            }
        })
    }
}
