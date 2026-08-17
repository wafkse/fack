//! Display implementation expansion.

use alloc::vec::Vec;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Generics, Ident};

use crate::{
    binding::Bind as _,
    bounds::{Context as BoundContext, Contribute as _},
    field::{FieldId, Fields},
    semantics::{Display, Variant},
};

use crate::format::ExpandContext as FormatContext;

use super::{Context, Expand};

/// One structure `Display` implementation request.
pub struct DisplayImpl<'target> {
    /// Error type identifier.
    name: &'target Ident,

    /// Error type generic parameters.
    generics: &'target Generics,

    /// Fields available to display generation.
    fields: &'target Fields,

    /// Validated display behavior.
    display: &'target Display,
}

impl<'target> DisplayImpl<'target> {
    /// Construct one structure display expansion request.
    pub const fn new(name: &'target Ident, generics: &'target Generics, fields: &'target Fields, display: &'target Display) -> Self {
        Self {
            name,
            generics,
            fields,
            display,
        }
    }
}

impl Expand for DisplayImpl<'_> {
    /// Shared generation options used by the display implementation.
    type Context = Context;

    /// Generate a structure `Display` implementation with validated bounds.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self {
            name,
            generics,
            fields,
            display,
        } = self;
        let root = context.root();
        let inline = context.inline();
        let mut generics = generics.clone();
        let bound_context = BoundContext::new(fields, root);

        display.contribute(&mut generics, &bound_context);

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let body = DisplayBody { fields, display }.expand_with(context.clone())?;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #root::fmt::Display for #name #ty_generics #where_clause {
                #inline
                fn fmt(&self, f: &mut #root::fmt::Formatter<'_>) -> #root::fmt::Result {
                    #body
                }
            }
        })
    }
}

/// One structure display method body.
struct DisplayBody<'target> {
    /// Fields available to the display body.
    fields: &'target Fields,

    /// Validated display behavior for the body.
    display: &'target Display,
}

impl Expand for DisplayBody<'_> {
    /// Shared generation options used by the display method body.
    type Context = Context;

    /// Generate the structure display body for the selected display behavior.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self { fields, display } = self;
        let root = context.root();

        match display {
            Display::Format(format) => {
                let selected = if format.arguments().is_empty() {
                    format.fields().to_vec()
                } else {
                    (0..fields.len()).map(FieldId::from_index).collect()
                };
                let bindings = fields.bind(&selected);
                let pattern = bindings.pattern();
                let body = format.expand_with(FormatContext::new(root.clone()))?;

                Ok(quote! {
                    let &Self #pattern = self;
                    #body
                })
            }
            Display::Transparent(field) => {
                let bindings = fields.bind(&[*field]);
                let pattern = bindings.pattern();
                let binding = bindings.ident(*field);

                Ok(quote! {
                    let &Self #pattern = self;
                    #root::fmt::Display::fmt(#binding, f)
                })
            }
            Display::Custom(path) => Ok(quote! { #path(self, f) }),
        }
    }
}

/// One enumeration `Display` implementation request.
pub struct EnumDisplayImpl<'target> {
    /// Enumeration identifier.
    name: &'target Ident,

    /// Enumeration generic parameters.
    generics: &'target Generics,

    /// Validated variants used to generate match arms.
    variants: &'target [Variant],
}

impl<'target> EnumDisplayImpl<'target> {
    /// Construct one enumeration display expansion request.
    pub const fn new(name: &'target Ident, generics: &'target Generics, variants: &'target [Variant]) -> Self {
        Self { name, generics, variants }
    }
}

impl Expand for EnumDisplayImpl<'_> {
    /// Shared generation options used by the enum display implementation.
    type Context = Context;

    /// Generate an enum `Display` implementation and all variant match arms.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self { name, generics, variants } = self;
        let root = context.root();
        let inline = context.inline();
        let mut generics = generics.clone();

        for variant in variants {
            let bound_context = BoundContext::new(variant.fields(), root);

            variant.display().contribute(&mut generics, &bound_context);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let arms = variants
            .iter()
            .map(|variant| VariantDisplayArm { variant }.expand_with(context.clone()))
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #root::fmt::Display for #name #ty_generics #where_clause {
                #inline
                fn fmt(&self, f: &mut #root::fmt::Formatter<'_>) -> #root::fmt::Result {
                    match self {
                        #(#arms),*
                    }
                }
            }
        })
    }
}

/// One enumeration display match arm.
struct VariantDisplayArm<'target> {
    /// Validated variant represented by this arm.
    variant: &'target Variant,
}

impl Expand for VariantDisplayArm<'_> {
    /// Shared generation options used by the variant display arm.
    type Context = Context;

    /// Generate one display match arm using only bindings required by that
    /// variant.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self { variant } = self;
        let name = variant.name();
        let fields = variant.fields();
        let root = context.root();

        match variant.display() {
            Display::Format(format) => {
                let selected = if format.arguments().is_empty() {
                    format.fields().to_vec()
                } else {
                    (0..fields.len()).map(FieldId::from_index).collect()
                };
                let bindings = fields.bind(&selected);
                let pattern = bindings.pattern();
                let body = format.expand_with(FormatContext::new(root.clone()))?;

                Ok(quote! { &Self::#name #pattern => { #body } })
            }
            Display::Transparent(field) => {
                let bindings = fields.bind(&[*field]);
                let pattern = bindings.pattern();
                let binding = bindings.ident(*field);

                Ok(quote! {
                    &Self::#name #pattern => #root::fmt::Display::fmt(#binding, f)
                })
            }
            Display::Custom(path) => {
                let bindings = fields.bind(&[]);
                let pattern = bindings.pattern();

                Ok(quote! { &Self::#name #pattern => #path(self, f) })
            }
        }
    }
}
