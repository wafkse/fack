//! Automatic `From` implementation expansion.
//!
//! Each validated conversion is wrapped in one private expansion request. The
//! request selects either a structure constructor or one enum variant while the
//! shared `Expand` contract supplies global generation context.

use proc_macro2::TokenStream;
use quote::quote;

use crate::semantic::Variant;

use super::{Context, ConversionExpansion, Expand};

impl<'target> Expand for ConversionExpansion<'target> {
    type Context = &'target Context;
    type Output = TokenStream;

    fn expand_with(self, context: Self::Context) -> syn::Result<Self::Output> {
        let Self {
            name,
            generics,
            variant,
            conversion,
        } = self;

        let inline = context.inline();

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let field = conversion.field();
        let field_type = field.ty();
        let variant = variant.map(Variant::name);
        let construct = match (variant, field.name()) {
            (None, Some(field)) => quote! { Self { #field: value } },
            (None, None) => quote! { Self(value) },
            (Some(variant), Some(field)) => quote! { Self::#variant { #field: value } },
            (Some(variant), None) => quote! { Self::#variant(value) },
        };

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics From<#field_type> for #name #ty_generics #where_clause {
                #inline
                fn from(value: #field_type) -> Self {
                    #construct
                }
            }
        })
    }
}
