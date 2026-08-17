//! Generic bound contribution from validated semantics.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Generics, Type, WherePredicate};

use crate::{
    field::Fields,
    format::FormatTrait,
    semantics::{Display, ErrorSource},
};

/// Context required to derive bounds from field-based semantics.
pub struct Context<'context> {
    /// Fields whose types may require generated bounds.
    fields: &'context Fields,

    /// Generated path root used by bound predicates.
    root: &'context TokenStream,
}

impl<'context> Context<'context> {
    /// Construct a bound contribution context.
    #[inline]
    #[must_use]
    pub const fn new(fields: &'context Fields, root: &'context TokenStream) -> Self {
        Self { fields, root }
    }

    /// Return the field collection.
    #[inline]
    #[must_use]
    pub const fn fields(&self) -> &Fields {
        let Self { fields, .. } = self;

        fields
    }

    /// Return the generated root path tokens.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> &TokenStream {
        let Self { root, .. } = self;

        root
    }
}

/// Contribute generic predicates required by one semantic capability.
pub trait Contribute<ContextType: ?Sized> {
    /// Add required predicates to the supplied generics.
    fn contribute(&self, generics: &mut Generics, context: &ContextType);
}

impl Contribute<Context<'_>> for Display {
    /// Add only the formatting predicates required by the validated display
    /// behavior.
    #[inline]
    fn contribute(&self, generics: &mut Generics, context: &Context<'_>) {
        let fields = context.fields();
        let root = context.root();

        match self {
            Self::Format(format) => {
                for format_use in format.uses() {
                    let ty = fields.ty(format_use.field());

                    if let Some(trait_name) = TraitName::format(format_use.format_trait()) {
                        Predicate::push(generics, quote! { #ty: #root::fmt::#trait_name });
                    }
                }
            }
            Self::Transparent(field) => {
                let ty = fields.ty(*field);

                Predicate::push(generics, quote! { #ty: #root::fmt::Display });
            }
            Self::Custom(..) => {}
        }
    }
}

impl Contribute<Context<'_>> for ErrorSource {
    /// Add the error bound required by the resolved source type when one
    /// exists.
    #[inline]
    fn contribute(&self, generics: &mut Generics, context: &Context<'_>) {
        let root = context.root();
        let source = match self {
            Self::Field(source) | Self::Transparent(source) => source,
            Self::None => return,
        };
        let ty = source.error_type();

        if !matches!(ty, Type::TraitObject(..)) {
            Predicate::push(generics, quote! { #ty: #root::error::Error + 'static });
        }
    }
}

/// Trait-name mapping for Rust formatting modes.
struct TraitName;

impl TraitName {
    /// Map a validated format mode to the corresponding Rust formatting trait.
    fn format(format_trait: FormatTrait) -> Option<syn::Ident> {
        let name = match format_trait {
            FormatTrait::Display => "Display",
            FormatTrait::Debug => "Debug",
            FormatTrait::LowerHex => "LowerHex",
            FormatTrait::UpperHex => "UpperHex",
            FormatTrait::Octal => "Octal",
            FormatTrait::Binary => "Binary",
            FormatTrait::LowerExp => "LowerExp",
            FormatTrait::UpperExp => "UpperExp",
            FormatTrait::Pointer => return None,
        };

        Some(syn::Ident::new(name, proc_macro2::Span::call_site()))
    }
}

/// Construction of generated where predicates.
struct Predicate;

impl Predicate {
    /// Parse and append one generated where predicate.
    fn push(generics: &mut Generics, tokens: TokenStream) {
        let predicate = syn::parse2::<WherePredicate>(tokens).expect("fack generates only syntactically valid where predicates");

        generics.make_where_clause().predicates.push(predicate);
    }

    /// Add the `Error` supertrait requirements for the generated type itself.
    pub fn self_error(generics: &mut Generics, root: &TokenStream) {
        Self::push(generics, quote! { Self: #root::fmt::Debug + #root::fmt::Display });
    }
}

/// Contribution of the generated type's own `Error` supertrait requirements.
#[derive(Clone, Copy, Debug, Default)]
pub struct ErrorSelf;

impl Contribute<TokenStream> for ErrorSelf {
    /// Add the `Debug` and `Display` supertrait requirements of `Error`.
    #[inline]
    fn contribute(&self, generics: &mut Generics, root: &TokenStream) {
        Predicate::self_error(generics, root);
    }
}
