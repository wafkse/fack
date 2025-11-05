//! Shared error characteristics for the error type generation process.

use alloc::{string::ToString, vec::Vec};

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;

use syn::{
    Expr, Ident, LitInt, LitStr, Path, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
};

use super::ErrorList;

/// A meta-parameter for an error type.
///
/// This is the conjuction of all meta-parameters that are used in the error
/// type generation process.
///
/// This is used to parse the meta-parameters of an error type, to then be
/// processed accordingly to the position in where they were found.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Param {
    /// The identifier of the meta-parameter.
    ///
    /// In most cases, this is the identifier which is used to identify which
    /// [`ParamKind`] to parse.
    pub name: Option<Ident>,

    /// The parameter kind of the meta-parameter.
    pub kind: ParamKind,
}

impl Param {
    /// A new meta-parameter that has no specific name.
    #[inline]
    #[must_use]
    pub const fn lone(kind: ParamKind) -> Self {
        Self { name: None, kind }
    }

    /// A new meta-parameter that has a specific name.
    #[inline]
    #[must_use]
    pub const fn identified(name: Ident, kind: ParamKind) -> Self {
        Self { name: Some(name), kind }
    }
}

impl Param {
    /// Classify an iterator of borrowed [`syn::Attribute`]s into a list of
    /// [`Param`]s where it is deemed possible.
    pub fn classify<'a>(iter: impl IntoIterator<Item = &'a syn::Attribute>) -> syn::Result<(Vec<Self>, Vec<&'a syn::Attribute>)> {
        let attr_iter = iter.into_iter();

        let (attr_len, ..) = attr_iter.size_hint();

        let mut param_list = Vec::with_capacity(attr_len);

        let mut attr_list = Vec::new();

        let mut error_list = ErrorList::empty();

        for attr in attr_iter {
            if let Some(ident) = attr.path().get_ident() {
                if ident == "error" {
                    match attr.parse_args_with(Param::parse) {
                        Ok(param) => param_list.push(param),
                        Err(error) => error_list.append(error),
                    }
                } else {
                    attr_list.push(attr);
                }
            } else {
                attr_list.push(attr);
            }
        }

        error_list.compose((param_list, attr_list))
    }
}

impl Parse for Param {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Ident) {
            let ident: Ident = input.parse()?;

            let param = if ident == "inline" {
                let inline_opts: InlineOptions = if input.peek(Paren) {
                    let inline_content;

                    syn::parenthesized!(inline_content in input);

                    inline_content.parse()?
                } else {
                    InlineOptions::default()
                };

                Param::identified(ident, ParamKind::Inline(inline_opts))
            } else if ident == "import" {
                let import_content;

                syn::parenthesized!(import_content in input);

                let import_root: ImportRoot = import_content.parse()?;

                Param::identified(ident, ParamKind::Import(import_root))
            } else if ident == "source" {
                let source_content;

                syn::parenthesized!(source_content in input);

                let source_field: FieldRef = source_content.parse()?;

                Param::identified(ident, ParamKind::Source(source_field))
            } else if ident == "transparent" {
                let trans_content;

                syn::parenthesized!(trans_content in input);

                let source_field: Transparent = trans_content.parse()?;

                Param::identified(ident, ParamKind::Transparent(source_field))
            } else if ident == "from" {
                Param::identified(ident, ParamKind::From)
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "expected `inline`, `import`, `source`, `transparent`, or `from`",
                ));
            };

            Ok(param)
        } else if lookahead.peek(LitStr) {
            Ok(Param::lone(ParamKind::Format(Format::parse(input)?)))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Param {
    /// Attempt to parse a meta-parameter from a [`syn::Meta`].
    ///
    /// Note that will reinterpret the supplied [`syn::Meta`] as a [`Param`]
    /// using [`Param::parse`].
    #[inline]
    pub fn attribute(target_meta: &syn::Meta) -> syn::Result<Self> {
        syn::parse2::<Self>(target_meta.to_token_stream()).map_err(|error| syn::Error::new_spanned(target_meta, error))
    }
}

/// The kind of meta-parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamKind {
    /// The inline behavior of the error type.
    Inline(InlineOptions),

    /// The import root of the error type.
    Import(ImportRoot),

    /// The format string of the error type.
    Format(Format),

    /// The source-forwarding behavior of the error type.
    Source(FieldRef),

    /// A transparent error type.
    ///
    /// This is useful for new-type wrappers that are transparent over some
    /// other type.
    Transparent(Transparent),

    /// The from forwarding behavior of the error type.
    ///
    /// This is useful for new-type wrappers that are completely opaque to the
    /// error type.
    From,
}

/// The inline options for an error type.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Copy)]
pub enum InlineOptions {
    /// Neutral inline behavior.
    ///
    /// This will mark all implemented methods as `#[inline]`, which will hint
    /// the compiler to inline them if it deems it beneficial.
    ///
    /// This is the default behavior.
    #[default]
    Neutral,

    /// Never attempt to inline implemented methods for this error type.
    Never,

    /// Always attempt to inline implemented methods for this error type.
    Always,
}

impl Parse for InlineOptions {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;

        match ident.to_string().as_str() {
            "neutral" => Ok(Self::Neutral),
            "never" => Ok(Self::Never),
            "always" => Ok(Self::Always),
            _ => Err(syn::Error::new_spanned(ident, "expected `neutral`, `never` or `always`")),
        }
    }
}

/// The root of all imports for error-related types.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImportRoot(pub Path);

impl Parse for ImportRoot {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;

        Ok(Self(path))
    }
}

/// The format string for an error's `Display` implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Format {
    /// The format string of the error variant.
    pub format: LitStr,

    /// The format arguments of the error variant.
    pub format_args: Punctuated<Expr, Token![,]>,
}

impl Parse for Format {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let format: LitStr = input.parse()?;

        let format_args = if input.peek(Token![,]) {
            let _ = input.parse::<Token![,]>()?;

            Punctuated::<Expr, Token![,]>::parse_terminated(input)?
        } else {
            Punctuated::new()
        };

        Ok(Self { format, format_args })
    }
}

/// A reference to a field in any structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldRef {
    /// A reference to a field by its distinctive name.
    Named(Ident),

    /// A reference to an unnamed field by its index.
    Indexed(usize),
}

impl Parse for FieldRef {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Ident) {
            let ident: Ident = input.parse()?;

            Ok(Self::Named(ident))
        } else if lookahead.peek(LitInt) {
            let lit_int: LitInt = input.parse()?;

            let index = lit_int
                .base10_parse::<usize>()
                .map_err(|_| syn::Error::new_spanned(lit_int, "expected valid field index"))?;

            Ok(Self::Indexed(index))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for FieldRef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            FieldRef::Named(ident) => ident.to_tokens(tokens),
            FieldRef::Indexed(index) => LitInt::new(index.to_string().as_str(), Span::call_site()).to_tokens(tokens),
        }
    }
}

/// An enumeration that determines whether a field is the source of the error or
/// not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ForwardSource {
    /// No forwarding.
    ///
    /// This is the default.
    #[default]
    No,

    /// Yes, forward to the field.
    Yes,
}

impl Parse for ForwardSource {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;

        if ident == "forward" {
            Ok(Self::Yes)
        } else {
            Err(syn::Error::new_spanned(
                ident,
                "expected just `source`, no internal further parameters",
            ))
        }
    }
}

/// A structure that determines transparence over a field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transparent(pub FieldRef);

impl Parse for Transparent {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let field_ref: FieldRef = input.parse()?;

        Ok(Self(field_ref))
    }
}
