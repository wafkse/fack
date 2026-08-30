//! Display implementation expansion.
//!
//! Structure bodies, enum arms, format rendering, and bound contribution are
//! separate expansion nodes under the shared parent contract. Each node carries
//! only the validated state required for its one operation.

use alloc::vec::Vec;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Generics;

use crate::{
    input::Fields,
    semantic::{BindingRequirement, Display, FormatTrait, Resolved, Variant},
};

use super::{Context, DisplayExpansion, Expand, FieldBinding, FieldPattern, FieldSelection, Predicate};

impl<'target> Expand for DisplayExpansion<'target> {
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
                display,
            } => {
                let generics = DisplayBoundExpansion {
                    subject: DisplayBoundSubject::One(display),
                    root,
                }
                .expand_with(generics.clone())?;

                let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

                let body = DisplayBody { fields, display }.expand_with(context)?;

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
            Self::Enumeration { name, generics, variants } => {
                let generics = DisplayBoundExpansion {
                    subject: DisplayBoundSubject::Variants(variants),
                    root,
                }
                .expand_with(generics.clone())?;

                let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

                let arms = variants
                    .iter()
                    .map(|variant| VariantDisplayArm { variant }.expand_with(context))
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
    }
}

/// Display state whose generic requirements are being expanded.
enum DisplayBoundSubject<'target> {
    /// One structure display state.
    One(&'target Display),

    /// Every display state in one enumeration.
    Variants(&'target [Variant]),
}

/// Expansion node for display generic requirements.
// NOTE(invariant): Every display subject has completed semantic validation
// before bound generation starts.
struct DisplayBoundExpansion<'target> {
    /// Validated display state receiving generated bounds.
    subject: DisplayBoundSubject<'target>,

    /// Generated root path used by formatting predicates.
    root: &'target TokenStream,
}

impl DisplayBoundExpansion<'_> {
    /// Add the predicates required by one display state.
    fn contribute(display: &Display, mut generics: Generics, root: &TokenStream) -> syn::Result<Generics> {
        match display {
            &Display::Format(ref format) => {
                for format_use in format.uses() {
                    let ty = format_use.field().ty();

                    match TraitName::format(format_use.format_trait()) {
                        Some(trait_name) => {
                            generics = Predicate(quote! { #ty: #root::fmt::#trait_name }).expand_with(generics)?;
                        }
                        None => {}
                    }
                }

                Ok(generics)
            }
            &Display::Transparent(ref field) => {
                let ty = field.ty();

                Predicate(quote! { #ty: #root::fmt::Display }).expand_with(generics)
            }
            &Display::Custom(..) => Ok(generics),
        }
    }
}

impl Expand for DisplayBoundExpansion<'_> {
    type Context = Generics;
    type Output = Generics;

    fn expand_with(self, mut generics: Self::Context) -> syn::Result<Self::Output> {
        let Self { subject, root } = self;

        match subject {
            DisplayBoundSubject::One(display) => Self::contribute(display, generics, root),
            DisplayBoundSubject::Variants(variants) => {
                for variant in variants {
                    generics = Self::contribute(variant.display(), generics, root)?;
                }

                Ok(generics)
            }
        }
    }
}

/// One structure display method body.
// NOTE(invariant): The display semantics were validated against the borrowed
// field collection before construction.
struct DisplayBody<'target> {
    /// Fields available to the display body.
    fields: &'target Fields,

    /// Validated display behavior for the body.
    display: &'target Display,
}

impl<'target> Expand for DisplayBody<'target> {
    type Context = &'target Context;
    type Output = TokenStream;

    fn expand_with(self, context: Self::Context) -> syn::Result<Self::Output> {
        let Self { fields, display } = self;

        let root = context.root();

        match display {
            &Display::Format(ref format) => match format {
                &Resolved::Static(_) => FormatExpansion(format, root).expand(),
                &Resolved::Dynamic(ref dynamic) => {
                    let pattern = match dynamic.bindings() {
                        &BindingRequirement::Selected(ref selected) => {
                            FieldPattern(fields, FieldSelection::Selected(selected.as_slice())).expand()?
                        }
                        &BindingRequirement::All => FieldPattern(fields, FieldSelection::All).expand()?,
                    };
                    let body = FormatExpansion(format, root).expand()?;

                    Ok(quote! {
                        let &Self #pattern = self;
                        #body
                    })
                }
            },
            &Display::Transparent(ref field) => {
                let binding = FieldBinding(fields, field).expand()?;
                let pattern = binding.pattern();
                let binding = binding.ident();

                Ok(quote! {
                    let &Self #pattern = self;
                    #root::fmt::Display::fmt(#binding, f)
                })
            }
            &Display::Custom(ref path) => Ok(quote! { #path(self, f) }),
        }
    }
}

/// One enumeration display match arm.
// NOTE(invariant): The borrowed variant is a validated member of the enclosing
// enumeration.
struct VariantDisplayArm<'target> {
    /// Validated variant represented by this arm.
    variant: &'target Variant,
}

impl<'target> Expand for VariantDisplayArm<'target> {
    type Context = &'target Context;
    type Output = TokenStream;

    fn expand_with(self, context: Self::Context) -> syn::Result<Self::Output> {
        let Self { variant } = self;

        let name = variant.name();
        let fields = variant.fields();
        let root = context.root();

        match variant.display() {
            &Display::Format(ref format) => {
                let pattern = match format {
                    &Resolved::Static(_) => FieldPattern(fields, FieldSelection::Selected(&[])).expand()?,
                    &Resolved::Dynamic(ref dynamic) => match dynamic.bindings() {
                        &BindingRequirement::Selected(ref selected) => {
                            FieldPattern(fields, FieldSelection::Selected(selected.as_slice())).expand()?
                        }
                        &BindingRequirement::All => FieldPattern(fields, FieldSelection::All).expand()?,
                    },
                };
                let body = FormatExpansion(format, root).expand()?;

                Ok(quote! { &Self::#name #pattern => { #body } })
            }
            &Display::Transparent(ref field) => {
                let binding = FieldBinding(fields, field).expand()?;
                let pattern = binding.pattern();
                let binding = binding.ident();

                Ok(quote! {
                    &Self::#name #pattern => #root::fmt::Display::fmt(#binding, f)
                })
            }
            &Display::Custom(ref path) => {
                let pattern = FieldPattern(fields, FieldSelection::Selected(&[])).expand()?;

                Ok(quote! { &Self::#name #pattern => #path(self, f) })
            }
        }
    }
}

/// Expansion node for one validated format body.
// NOTE(invariant): The format was fully resolved before token expansion.
struct FormatExpansion<'target>(&'target Resolved, &'target TokenStream);

impl Expand for FormatExpansion<'_> {
    type Context = ();
    type Output = TokenStream;

    fn expand_with(self, (): Self::Context) -> syn::Result<Self::Output> {
        let Self(format, root) = self;

        let tokens = match format {
            &Resolved::Static(ref literal) => quote! { f.write_str(#literal) },
            &Resolved::Dynamic(ref format) => {
                let literal = format.literal();
                let arguments = format.arguments();

                quote! { #root::write!(f, #literal, #arguments) }
            }
        };

        Ok(tokens)
    }
}

/// Trait name mapping for Rust formatting modes.
struct TraitName;

impl TraitName {
    /// Map a validated format mode to the corresponding Rust formatting trait.
    fn format(format_trait: FormatTrait) -> Option<syn::Ident> {
        let name = match format_trait {
            FormatTrait::Display => Some("Display"),
            FormatTrait::Debug => Some("Debug"),
            FormatTrait::LowerHex => Some("LowerHex"),
            FormatTrait::UpperHex => Some("UpperHex"),
            FormatTrait::Octal => Some("Octal"),
            FormatTrait::Binary => Some("Binary"),
            FormatTrait::LowerExp => Some("LowerExp"),
            FormatTrait::UpperExp => Some("UpperExp"),
            FormatTrait::Pointer => None,
        };

        name.map(|name| syn::Ident::new(name, proc_macro2::Span::call_site()))
    }
}
