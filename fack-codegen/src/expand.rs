//! Expansion of validated semantics into Rust trait implementations.
//!
//! This subsystem owns final token generation only. Every expansion node uses
//! one private `Expand` contract with an explicit context and output type. Small
//! request values distinguish operations without exposing sibling APIs.

use alloc::vec::Vec;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Generics, Ident, WherePredicate};

use crate::{
    input::{Field, Fields, ImportRoot, InlinePolicy},
    semantic::{Conversion, Display, Enumeration, ErrorSource, Structure, Target, Variant},
};

mod binding;
mod conversion;
mod display;
mod error;

/// Expand one fully validated semantic target.
pub fn target(target: Target) -> syn::Result<TokenStream> {
    target.expand()
}

/// Expansion of one validated node with explicit operation context.
trait Expand {
    /// External context required by this expansion node.
    type Context;

    /// Value produced after expansion completes.
    type Output;

    /// Expand this node using explicit context.
    fn expand_with(self, context: Self::Context) -> syn::Result<Self::Output>;

    /// Expand this node using the default context.
    fn expand(self) -> syn::Result<Self::Output>
    where
        Self::Context: Default,
        Self: Sized,
    {
        Self::expand_with(self, Self::Context::default())
    }
}

impl Expand for Target {
    type Context = ();
    type Output = TokenStream;

    fn expand_with(self, (): Self::Context) -> syn::Result<Self::Output> {
        match self {
            Self::Struct(structure) => (*structure).expand(),
            Self::Enum(enumeration) => enumeration.expand(),
        }
    }
}

impl Expand for Structure {
    type Context = ();
    type Output = TokenStream;

    fn expand_with(self, (): Self::Context) -> syn::Result<Self::Output> {
        let (header, fields, display, source, conversion) = self.parts();

        let (inline, import, name, generics) = header.parts();

        let context = Context::new(inline, import);
        let display = DisplayExpansion::Structure {
            name: &name,
            generics: &generics,
            fields: &fields,
            display: &display,
        }
        .expand_with(&context)?;
        let error = ErrorExpansion::Structure {
            name: &name,
            generics: &generics,
            fields: &fields,
            source: &source,
        }
        .expand_with(&context)?;
        let conversion = conversion
            .as_ref()
            .map(|conversion| ConversionExpansion {
                name: &name,
                generics: &generics,
                variant: None,
                conversion,
            })
            .map(|conversion| conversion.expand_with(&context))
            .transpose()?;

        Ok(quote! {
            #conversion
            #display
            #error
        })
    }
}

impl Expand for Enumeration {
    type Context = ();
    type Output = TokenStream;

    fn expand_with(self, (): Self::Context) -> syn::Result<Self::Output> {
        let (header, variants) = self.parts();

        let (inline, import, name, generics) = header.parts();

        let context = Context::new(inline, import);
        let display = DisplayExpansion::Enumeration {
            name: &name,
            generics: &generics,
            variants: &variants,
        }
        .expand_with(&context)?;
        let error = ErrorExpansion::Enumeration {
            name: &name,
            generics: &generics,
            variants: &variants,
        }
        .expand_with(&context)?;
        let conversions = variants
            .iter()
            .filter_map(|variant| {
                variant.conversion().map(|conversion| ConversionExpansion {
                    name: &name,
                    generics: &generics,
                    variant: Some(variant),
                    conversion,
                })
            })
            .map(|conversion| conversion.expand_with(&context))
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            #(#conversions)*
            #display
            #error
        })
    }
}

/// Pattern and identifier established for one resolved field.
#[derive(Clone, Debug)]
// NOTE(invariant): The pattern establishes the stored identifier for the same
// resolved field.
struct Binding {
    /// Generated destructuring pattern.
    pattern: TokenStream,

    /// Local identifier established by the pattern.
    ident: Ident,
}

impl Binding {
    /// Return the generated destructuring pattern.
    #[inline]
    #[must_use]
    const fn pattern(&self) -> &TokenStream {
        let &Self { ref pattern, .. } = self;

        pattern
    }

    /// Return the local identifier established by the pattern.
    #[inline]
    #[must_use]
    const fn ident(&self) -> &Ident {
        let &Self { ref ident, .. } = self;

        ident
    }
}

/// Expansion request that binds one resolved field.
// NOTE(invariant): The field was resolved from the paired field collection.
struct FieldBinding<'field>(&'field Fields, &'field Field);

/// Expansion request for one structural field pattern.
// NOTE(invariant): Selected fields were resolved from the paired field
// collection before this request is constructed.
struct FieldPattern<'field>(&'field Fields, FieldSelection<'field>);

/// Field selection used by one pattern expansion request.
enum FieldSelection<'field> {
    /// Bind exactly these resolved fields.
    Selected(&'field [Field]),

    /// Bind every declared field.
    All,
}

/// Expansion request for one automatic conversion implementation.
// NOTE(invariant): The conversion belongs to the target or variant named by
// this request.
struct ConversionExpansion<'target> {
    /// Generated error type identifier.
    name: &'target Ident,

    /// Generic parameters of the generated error type.
    generics: &'target Generics,

    /// Optional enum variant receiving the converted value.
    variant: Option<&'target Variant>,

    /// Validated automatic conversion semantics.
    conversion: &'target Conversion,
}

/// Expansion request for generated `Display` behavior.
enum DisplayExpansion<'target> {
    /// Generate `Display` for one structure.
    Structure {
        /// Generated structure identifier.
        name: &'target Ident,

        /// Generic parameters of the structure.
        generics: &'target Generics,

        /// Validated structural fields.
        fields: &'target Fields,

        /// Validated display semantics.
        display: &'target Display,
    },

    /// Generate `Display` for one enumeration.
    Enumeration {
        /// Generated enumeration identifier.
        name: &'target Ident,

        /// Generic parameters of the enumeration.
        generics: &'target Generics,

        /// Validated enumeration variants.
        variants: &'target [Variant],
    },
}

/// Expansion request for generated `Error` behavior.
enum ErrorExpansion<'target> {
    /// Generate `Error` for one structure.
    Structure {
        /// Generated structure identifier.
        name: &'target Ident,

        /// Generic parameters of the structure.
        generics: &'target Generics,

        /// Validated structural fields.
        fields: &'target Fields,

        /// Validated source semantics.
        source: &'target ErrorSource,
    },

    /// Generate `Error` for one enumeration.
    Enumeration {
        /// Generated enumeration identifier.
        name: &'target Ident,

        /// Generic parameters of the enumeration.
        generics: &'target Generics,

        /// Validated enumeration variants.
        variants: &'target [Variant],
    },
}

/// Expansion node that appends one generated where predicate.
// NOTE(invariant): The stored tokens represent one predicate that is parsed
// before it is inserted into generic state.
struct Predicate(TokenStream);

impl Expand for Predicate {
    type Context = Generics;
    type Output = Generics;

    fn expand_with(self, mut generics: Self::Context) -> syn::Result<Self::Output> {
        let Self(tokens) = self;

        let predicate = syn::parse2::<WherePredicate>(tokens)?;

        generics.make_where_clause().predicates.push(predicate);

        Ok(generics)
    }
}

/// Shared immutable inputs for generated trait implementations.
#[derive(Clone, Debug)]
// NOTE(invariant): Both token streams are derived once from validated global
// generation options.
struct Context {
    /// Rendered inline attribute for generated methods.
    inline: TokenStream,

    /// Generated path root used by implementations.
    root: TokenStream,
}

impl Context {
    /// Construct generation context from validated global options.
    #[inline]
    fn new(inline: InlinePolicy, import: ImportRoot) -> Self {
        let inline = match inline {
            InlinePolicy::Unspecified => TokenStream::new(),
            InlinePolicy::Neutral => quote! { #[inline] },
            InlinePolicy::Always => quote! { #[inline(always)] },
            InlinePolicy::Never => quote! { #[inline(never)] },
        };
        let root = match import {
            ImportRoot::Core => quote! { ::core },
            ImportRoot::Explicit(path) => path.to_token_stream(),
        };

        Self { inline, root }
    }

    /// Return the generated inline attribute.
    #[inline]
    #[must_use]
    const fn inline(&self) -> &TokenStream {
        let &Self { ref inline, .. } = self;

        inline
    }

    /// Return the generated root path.
    #[inline]
    #[must_use]
    const fn root(&self) -> &TokenStream {
        let &Self { ref root, .. } = self;

        root
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::{String, ToString};

    use proc_macro2::Span;

    use crate::{input, semantic};

    use super::target;

    /// Expand one derive input through input and semantic validation.
    fn expand(source: &str) -> syn::Result<String> {
        let input = syn::parse_str(source)?;
        let input = input::Target::parse(&input)?;
        let semantic = semantic::Target::from_input(input)?;
        let tokens = target(semantic)?;

        Ok(tokens.to_string())
    }

    /// Return a diagnostic when a test requirement is not satisfied.
    fn verify(condition: bool, message: &'static str) -> syn::Result<()> {
        match condition {
            true => Ok(()),
            false => Err(syn::Error::new(Span::call_site(), message)),
        }
    }

    /// Static messages use direct formatter string writes.
    #[test]
    fn static_messages_use_formatter_write_str() -> syn::Result<()> {
        let output = expand(
            r#"
            #[error("static message")]
            struct Static;
            "#,
        )?;

        verify(output.contains("write_str"), "static display must call write_str")?;
        verify(!output.contains("write !"), "static display must not call write")?;

        Ok(())
    }

    /// Source-less errors do not generate an explicit source method.
    #[test]
    fn source_less_errors_do_not_override_source() -> syn::Result<()> {
        let output = expand(
            r#"
            #[error("static message")]
            struct Static;
            "#,
        )?;

        verify(!output.contains("fn source"), "source-less errors must not override source")?;

        Ok(())
    }

    /// Ordinary and transparent sources generate distinct source expressions.
    #[test]
    fn ordinary_and_transparent_sources_expand_differently() -> syn::Result<()> {
        let ordinary = expand(
            r#"
            #[error("outer")]
            #[error(source(inner))]
            struct Outer { inner: std::io::Error }
            "#,
        )?;
        let transparent = expand("#[error(transparent(inner))]\nstruct Transparent { context: u8, inner: std::io::Error }")?;

        verify(ordinary.contains("Some"), "ordinary sources must return Some")?;
        verify(
            transparent.contains("Error :: source"),
            "transparent sources must delegate source lookup",
        )?;

        Ok(())
    }
}
