//! Error implementation and source-chain expansion.

use alloc::vec::Vec;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Generics, Ident};

use crate::{
    binding::Bind as _,
    bounds::{self, Context as BoundContext, Contribute as _},
    field::Fields,
    semantics::{ErrorSource, Variant},
};

use crate::source::ExpandContext as SourceContext;

use super::{Context, Expand};

/// One structure `Error` implementation request.
pub struct ErrorImpl<'target> {
    /// Error type identifier.
    name: &'target Ident,

    /// Error type generic parameters.
    generics: &'target Generics,

    /// Fields available to source generation.
    fields: &'target Fields,

    /// Validated source-chain behavior.
    source: &'target ErrorSource,
}

impl<'target> ErrorImpl<'target> {
    /// Construct one structure error expansion request.
    pub const fn new(name: &'target Ident, generics: &'target Generics, fields: &'target Fields, source: &'target ErrorSource) -> Self {
        Self {
            name,
            generics,
            fields,
            source,
        }
    }
}

impl Expand for ErrorImpl<'_> {
    /// Shared generation options used by the error implementation.
    type Context = Context;

    /// Generate a structure `Error` implementation and required source bounds.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self {
            name,
            generics,
            fields,
            source,
        } = self;
        let root = context.root();
        let mut generics = generics.clone();
        let bound_context = BoundContext::new(fields, root);

        bounds::ErrorSelf.contribute(&mut generics, root);
        source.contribute(&mut generics, &bound_context);

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let method = SourceMethod { fields, source }.expand_with(context.clone())?;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #root::error::Error for #name #ty_generics #where_clause {
                #method
            }
        })
    }
}

/// One generated `Error::source` method.
struct SourceMethod<'target> {
    /// Fields available to the generated source method.
    fields: &'target Fields,

    /// Validated source-chain behavior for the method.
    source: &'target ErrorSource,
}

impl Expand for SourceMethod<'_> {
    /// Shared generation options used by the source method.
    type Context = Context;

    /// Generate `Error::source` only when validated source semantics require
    /// it.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self { fields, source } = self;
        let root = context.root();
        let inline = context.inline();
        let (source, transparent) = match source {
            ErrorSource::None => return Ok(TokenStream::new()),
            ErrorSource::Field(source) => (source, false),
            ErrorSource::Transparent(source) => (source, true),
        };
        let bindings = fields.bind(&[source.field()]);
        let pattern = bindings.pattern();
        let binding = bindings.ident(source.field()).clone();
        let source_context = SourceContext::new(binding, root.clone(), transparent);
        let value = source.expand_with(source_context)?;

        Ok(quote! {
            #inline
            fn source(&self) -> Option<&(dyn #root::error::Error + 'static)> {
                let &Self #pattern = self;
                #value
            }
        })
    }
}

/// One enumeration `Error` implementation request.
pub struct EnumErrorImpl<'target> {
    /// Enumeration identifier.
    name: &'target Ident,

    /// Enumeration generic parameters.
    generics: &'target Generics,

    /// Validated variants used by source generation.
    variants: &'target [Variant],
}

impl<'target> EnumErrorImpl<'target> {
    /// Construct one enumeration error expansion request.
    pub const fn new(name: &'target Ident, generics: &'target Generics, variants: &'target [Variant]) -> Self {
        Self { name, generics, variants }
    }
}

impl Expand for EnumErrorImpl<'_> {
    /// Shared generation options used by the enum error implementation.
    type Context = Context;

    /// Generate the enum `Error` implementation and variant source dispatch.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self { name, generics, variants } = self;
        let root = context.root();
        let inline = context.inline();
        let mut generics = generics.clone();

        bounds::ErrorSelf.contribute(&mut generics, root);

        for variant in variants {
            let bound_context = BoundContext::new(variant.fields(), root);

            variant.source().contribute(&mut generics, &bound_context);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let has_source = variants.iter().any(|variant| !matches!(variant.source(), ErrorSource::None));
        let method = if has_source {
            let arms = variants
                .iter()
                .map(|variant| VariantSourceArm { variant }.expand_with(context.clone()))
                .collect::<syn::Result<Vec<_>>>()?;

            quote! {
                #inline
                fn source(&self) -> Option<&(dyn #root::error::Error + 'static)> {
                    match self {
                        #(#arms),*
                    }
                }
            }
        } else {
            TokenStream::new()
        };

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #root::error::Error for #name #ty_generics #where_clause {
                #method
            }
        })
    }
}

/// One enumeration source match arm.
struct VariantSourceArm<'target> {
    /// Validated variant represented by this source arm.
    variant: &'target Variant,
}

impl Expand for VariantSourceArm<'_> {
    /// Shared generation options used by one source match arm.
    type Context = Context;

    /// Generate one source match arm with the minimum required field binding.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let Self { variant } = self;
        let name = variant.name();
        let fields = variant.fields();
        let root = context.root();
        let (source, transparent) = match variant.source() {
            ErrorSource::None => {
                let bindings = fields.bind(&[]);
                let pattern = bindings.pattern();

                return Ok(quote! { &Self::#name #pattern => None });
            }
            ErrorSource::Field(source) => (source, false),
            ErrorSource::Transparent(source) => (source, true),
        };
        let bindings = fields.bind(&[source.field()]);
        let pattern = bindings.pattern();
        let binding = bindings.ident(source.field()).clone();
        let source_context = SourceContext::new(binding, root.clone(), transparent);
        let value = source.expand_with(source_context)?;

        Ok(quote! { &Self::#name #pattern => #value })
    }
}
