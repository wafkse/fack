//! Source formatting syntax accepted by `#[error(...)]`.
//!
//! The input format model preserves the literal and arbitrary Rust expressions
//! exactly as parsed. Capture interpretation belongs to semantic resolution,
//! while token rendering belongs to expansion.

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    Expr, Ident, LitStr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// A source-level Rust format string and explicit arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// NOTE(invariant): The literal and explicit arguments preserve one parsed format declaration without semantic rewriting.
pub struct Format {
    /// Source format string literal.
    literal: LitStr,

    /// Explicit Rust formatting arguments.
    arguments: Punctuated<Argument, Token![,]>,
}

impl Parse for Format {
    /// Parse the format literal together with any explicit Rust arguments.
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let literal = input.parse()?;
        let arguments = if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;

            Punctuated::<Argument, Token![,]>::parse_terminated(input)?
        } else {
            Punctuated::new()
        };

        Ok(Self { literal, arguments })
    }
}

/// One explicit Rust format argument.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// NOTE(invariant): The optional name and expression represent one parsed positional or named format argument.
pub struct Argument {
    /// Optional explicit named-argument identifier.
    name: Option<Ident>,

    /// Rust expression supplying the argument value.
    expression: Expr,
}

impl Parse for Argument {
    /// Parse one positional or explicitly named Rust format argument.
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = if input.peek(Ident) && input.peek2(Token![=]) && !input.peek2(Token![==]) {
            let name = input.parse()?;
            let _: Token![=] = input.parse()?;

            Some(name)
        } else {
            None
        };
        let expression = input.parse()?;

        Ok(Self { name, expression })
    }
}

impl ToTokens for Argument {
    /// Reconstruct the explicit argument syntax for the generated `write!`
    /// call.
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let &Self { ref name, ref expression } = self;

        match name.as_ref() {
            Some(name) => quote::quote!(#name = #expression).to_tokens(tokens),
            None => expression.to_tokens(tokens),
        }
    }
}

impl Format {
    /// Return the source literal span for diagnostics.
    #[inline]
    #[must_use]
    pub fn span(&self) -> Span {
        let &Self { ref literal, .. } = self;

        literal.span()
    }

    /// Consume this source format into its literal and explicit arguments.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (LitStr, Punctuated<Argument, Token![,]>) {
        let Self { literal, arguments } = self;

        (literal, arguments)
    }
}

impl Argument {
    /// Return the explicit argument name when one was supplied.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> Option<&Ident> {
        let &Self { ref name, .. } = self;

        name.as_ref()
    }
}
