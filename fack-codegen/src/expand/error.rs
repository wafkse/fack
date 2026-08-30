//! Error implementation and source-chain expansion.
//!
//! Error implementations, source methods, source expressions, and bound
//! contribution are expansion nodes under the shared parent contract. Validated
//! field proofs are consumed without source-level lookup.

use alloc::vec::Vec;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Generics, Ident, Type};

use crate::{
    input::Fields,
    semantic::{ErrorSource, Source, TransparentSource, Variant},
};

use super::{Context, ErrorExpansion, Expand, FieldBinding, FieldPattern, FieldSelection, Predicate};

impl<'target> Expand for ErrorExpansion<'target> {
    type Context = &'target Context;
    type Output = TokenStream;

    fn expand_with(self, context: Self::Context) -> syn::Result<Self::Output> {
        let root = context.root();
        let inline = context.inline();

        match self {
            Self::Structure {
                name,
                generics,
                fields,
                source,
            } => {
                let generics = ErrorBoundExpansion {
                    subject: ErrorBoundSubject::One(source),
                    root,
                }
                .expand_with(generics.clone())?;

                let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

                let method = SourceMethod { fields, source }.expand_with(context)?;

                Ok(quote! {
                    #[automatically_derived]
                    impl #impl_generics #root::error::Error for #name #ty_generics #where_clause {
                        #method
                    }
                })
            }
            Self::Enumeration { name, generics, variants } => {
                let generics = ErrorBoundExpansion {
                    subject: ErrorBoundSubject::Variants(variants),
                    root,
                }
                .expand_with(generics.clone())?;

                let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

                let has_source = variants.iter().any(|variant| !matches!(variant.source(), &ErrorSource::None));
                let method = if has_source {
                    let arms = variants
                        .iter()
                        .map(|variant| VariantSourceArm { variant }.expand_with(context))
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
    }
}

/// Error source state whose generic requirements are being expanded.
enum ErrorBoundSubject<'target> {
    /// One structure source state.
    One(&'target ErrorSource),

    /// Every source state in one enumeration.
    Variants(&'target [Variant]),
}

/// Expansion node for error generic requirements.
// NOTE(invariant): Every source subject has completed semantic validation before
// bound generation starts.
struct ErrorBoundExpansion<'target> {
    /// Validated source state receiving generated bounds.
    subject: ErrorBoundSubject<'target>,

    /// Generated root path used by error predicates.
    root: &'target TokenStream,
}

impl ErrorBoundExpansion<'_> {
    /// Add the source predicate required by one error source state.
    fn contribute_source(source: &ErrorSource, generics: Generics, root: &TokenStream) -> syn::Result<Generics> {
        let ty = match source {
            &ErrorSource::Field(ref source) => Some(source.error_type()),
            &ErrorSource::Transparent(ref source) => Some(source.error_type()),
            &ErrorSource::None => None,
        };

        match ty.filter(|ty| !matches!(ty, &&Type::TraitObject(..))) {
            Some(ty) => Predicate(quote! { #ty: #root::error::Error + 'static }).expand_with(generics),
            None => Ok(generics),
        }
    }
}

impl Expand for ErrorBoundExpansion<'_> {
    type Context = Generics;
    type Output = Generics;

    fn expand_with(self, generics: Self::Context) -> syn::Result<Self::Output> {
        let Self { subject, root } = self;

        let mut generics = Predicate(quote! { Self: #root::fmt::Debug + #root::fmt::Display }).expand_with(generics)?;

        match subject {
            ErrorBoundSubject::One(source) => Self::contribute_source(source, generics, root),
            ErrorBoundSubject::Variants(variants) => {
                for variant in variants {
                    generics = Self::contribute_source(variant.source(), generics, root)?;
                }

                Ok(generics)
            }
        }
    }
}

/// One generated `Error::source` method.
// NOTE(invariant): The source semantics were validated against the borrowed
// field collection before construction.
struct SourceMethod<'target> {
    /// Fields available to the generated source method.
    fields: &'target Fields,

    /// Validated source-chain behavior for the method.
    source: &'target ErrorSource,
}

impl<'target> Expand for SourceMethod<'target> {
    type Context = &'target Context;
    type Output = TokenStream;

    fn expand_with(self, context: Self::Context) -> syn::Result<Self::Output> {
        let Self { fields, source } = self;

        let root = context.root();
        let inline = context.inline();

        match source {
            &ErrorSource::None => Ok(TokenStream::new()),
            &ErrorSource::Field(ref source) => {
                let field = source.field();
                let binding = FieldBinding(fields, field).expand()?;
                let pattern = binding.pattern();
                let ident = binding.ident();
                let value = SourceExpansion::Ordinary {
                    source,
                    binding: ident,
                    root,
                }
                .expand()?;

                Ok(quote! {
                    #inline
                    fn source(&self) -> Option<&(dyn #root::error::Error + 'static)> {
                        let &Self #pattern = self;
                        #value
                    }
                })
            }
            &ErrorSource::Transparent(ref source) => {
                let field = source.field();
                let binding = FieldBinding(fields, field).expand()?;
                let pattern = binding.pattern();
                let ident = binding.ident();
                let value = SourceExpansion::Transparent {
                    source,
                    binding: ident,
                    root,
                }
                .expand()?;

                Ok(quote! {
                    #inline
                    fn source(&self) -> Option<&(dyn #root::error::Error + 'static)> {
                        let &Self #pattern = self;
                        #value
                    }
                })
            }
        }
    }
}

/// One enumeration source match arm.
// NOTE(invariant): The borrowed variant is a validated member of the enclosing
// enumeration.
struct VariantSourceArm<'target> {
    /// Validated variant represented by this source arm.
    variant: &'target Variant,
}

impl<'target> Expand for VariantSourceArm<'target> {
    type Context = &'target Context;
    type Output = TokenStream;

    fn expand_with(self, context: Self::Context) -> syn::Result<Self::Output> {
        let Self { variant } = self;

        let name = variant.name();
        let fields = variant.fields();
        let root = context.root();

        match variant.source() {
            &ErrorSource::None => {
                let pattern = FieldPattern(fields, FieldSelection::Selected(&[])).expand()?;

                Ok(quote! { &Self::#name #pattern => None })
            }
            &ErrorSource::Field(ref source) => {
                let field = source.field();
                let binding = FieldBinding(fields, field).expand()?;
                let pattern = binding.pattern();
                let ident = binding.ident();
                let value = SourceExpansion::Ordinary {
                    source,
                    binding: ident,
                    root,
                }
                .expand()?;

                Ok(quote! { &Self::#name #pattern => #value })
            }
            &ErrorSource::Transparent(ref source) => {
                let field = source.field();
                let binding = FieldBinding(fields, field).expand()?;
                let pattern = binding.pattern();
                let ident = binding.ident();
                let value = SourceExpansion::Transparent {
                    source,
                    binding: ident,
                    root,
                }
                .expand()?;

                Ok(quote! { &Self::#name #pattern => #value })
            }
        }
    }
}

/// Expansion node for one generated source expression.
enum SourceExpansion<'target> {
    /// Expand an ordinary source expression.
    Ordinary {
        /// Validated ordinary source semantics.
        source: &'target Source,

        /// Local field binding established by the enclosing pattern.
        binding: &'target Ident,

        /// Generated root path used by error trait references.
        root: &'target TokenStream,
    },

    /// Expand a transparent source expression.
    Transparent {
        /// Validated transparent source semantics.
        source: &'target TransparentSource,

        /// Local field binding established by the enclosing pattern.
        binding: &'target Ident,

        /// Generated root path used by error trait references.
        root: &'target TokenStream,
    },
}

impl Expand for SourceExpansion<'_> {
    type Context = ();
    type Output = TokenStream;

    fn expand_with(self, (): Self::Context) -> syn::Result<Self::Output> {
        let tokens = match self {
            Self::Ordinary { source, binding, root } => match source {
                &Source::Direct(_) => quote! { Some(#binding) },
                &Source::Boxed(_) => quote! { Some(&**#binding) },
                &Source::Optional(_) => quote! {
                    #binding.as_ref().map(|source| source as &(dyn #root::error::Error + 'static))
                },
                &Source::OptionalBoxed(_) => quote! {
                    #binding.as_ref().map(|source| &**source as &(dyn #root::error::Error + 'static))
                },
            },
            Self::Transparent { source, binding, root } => match source {
                &TransparentSource::Direct(_) => quote! { #root::error::Error::source(#binding) },
                &TransparentSource::Boxed(_) => quote! { #root::error::Error::source(&**#binding) },
            },
        };

        Ok(tokens)
    }
}
