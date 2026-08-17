//! Expansion of validated semantics into Rust trait implementations.

use alloc::vec::Vec;

use crate::{
    semantics::{Enumeration, Structure, Target},
    syntax::{Import, Inline},
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

mod conversion;
mod display;
mod error;

use conversion::ConversionImpl;
use display::{DisplayImpl, EnumDisplayImpl};
use error::{EnumErrorImpl, ErrorImpl};

/// Expand one validated semantic node into Rust tokens.
pub trait Expand {
    /// Additional context required by this semantic node.
    type Context;

    /// Expand using explicit context.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if final token construction discovers an invalid
    /// generated Rust fragment.
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream>;

    /// Expand with a default context.
    ///
    /// # Errors
    ///
    /// Returns any diagnostic produced by [`Expand::expand_with`].
    #[inline]
    fn expand(self) -> syn::Result<TokenStream>
    where
        Self::Context: Default,
        Self: Sized,
    {
        self.expand_with(Self::Context::default())
    }
}

/// Expand one fully validated semantic target.
pub fn target(target: Target) -> syn::Result<TokenStream> {
    match target {
        Target::Struct(structure) => StructureExpansion(*structure).expand(),
        Target::Enum(enumeration) => EnumerationExpansion(enumeration).expand(),
    }
}

/// Expansion request for one validated structure.
struct StructureExpansion(Structure);

impl Expand for StructureExpansion {
    /// Structure expansion is self-contained after validation.
    type Context = ();

    /// Compose conversion, display, and error implementations for one
    /// structure.
    fn expand_with(self, (): Self::Context) -> syn::Result<TokenStream> {
        let Self(structure) = self;
        let (header, fields, display, source, conversion) = structure.parts();
        let (inline, import, name, generics) = header.parts();
        let context = Context::new(inline, import);
        let display = DisplayImpl::new(&name, &generics, &fields, &display).expand_with(context.clone())?;
        let error = ErrorImpl::new(&name, &generics, &fields, &source).expand_with(context.clone())?;
        let conversion = conversion
            .as_ref()
            .map(|conversion| ConversionImpl::structure(&name, &generics, &fields, conversion).expand_with(context.clone()))
            .transpose()?;

        Ok(quote! {
            #conversion
            #display
            #error
        })
    }
}

/// Expansion request for one validated enumeration.
struct EnumerationExpansion(Enumeration);

impl Expand for EnumerationExpansion {
    /// Enumeration expansion is self-contained after validation.
    type Context = ();

    /// Compose conversions, display, and error implementations for one
    /// enumeration.
    fn expand_with(self, (): Self::Context) -> syn::Result<TokenStream> {
        let Self(enumeration) = self;
        let (header, variants) = enumeration.parts();
        let (inline, import, name, generics) = header.parts();
        let context = Context::new(inline, import);
        let display = EnumDisplayImpl::new(&name, &generics, &variants).expand_with(context.clone())?;
        let error = EnumErrorImpl::new(&name, &generics, &variants).expand_with(context.clone())?;
        let conversions = variants
            .iter()
            .filter_map(|variant| {
                variant
                    .conversion()
                    .map(|conversion| ConversionImpl::variant(&name, &generics, variant, conversion).expand_with(context.clone()))
            })
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            #(#conversions)*
            #display
            #error
        })
    }
}

/// Shared immutable inputs for generated trait implementations.
#[derive(Clone, Debug)]
pub struct Context {
    /// Rendered inline attribute for generated methods.
    inline: TokenStream,

    /// Generated path root used by implementations.
    root: TokenStream,
}

impl Context {
    /// Construct generation context from validated global options.
    #[must_use]
    #[inline]
    fn new(inline: Option<Inline>, import: Option<Import>) -> Self {
        let inline = match inline {
            Some(Inline::Neutral) => quote! { #[inline] },
            Some(Inline::Always) => quote! { #[inline(always)] },
            Some(Inline::Never) => quote! { #[inline(never)] },
            None => TokenStream::new(),
        };
        let root = match import {
            Some(Import(path)) => path.to_token_stream(),
            None => quote! { ::core },
        };

        Self { inline, root }
    }

    /// Return the generated inline attribute.
    #[inline]
    #[must_use]
    const fn inline(&self) -> &TokenStream {
        let Self { inline, .. } = self;

        inline
    }

    /// Return the generated root path.
    #[inline]
    #[must_use]
    const fn root(&self) -> &TokenStream {
        let Self { root, .. } = self;

        root
    }
}
