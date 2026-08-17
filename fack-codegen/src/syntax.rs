//! Parsed `#[error(...)]` syntax that is independent of semantic validation.

use alloc::{string::ToString, vec::Vec};

use syn::{
    Ident, Path,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::{diagnostic::Errors, field::FieldRef, format::Format};

/// A parsed `#[error(...)]` parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Param {
    /// Optional identifier that introduced the parameter.
    name: Option<Ident>,

    /// Parsed parameter value.
    kind: ParamKind,
}

impl Param {
    /// Construct an unnamed parameter.
    #[inline]
    #[must_use]
    pub const fn lone(kind: ParamKind) -> Self {
        Self { name: None, kind }
    }

    /// Construct a named parameter.
    #[inline]
    #[must_use]
    pub const fn identified(name: Ident, kind: ParamKind) -> Self {
        Self { name: Some(name), kind }
    }

    /// Consume the parameter into its optional introducer and parsed value.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Option<Ident>, ParamKind) {
        let Self { name, kind } = self;

        (name, kind)
    }

    /// Classify `#[error(...)]` attributes and retain unrelated attributes.
    #[inline]
    pub fn classify<'a>(iter: impl IntoIterator<Item = &'a syn::Attribute>) -> syn::Result<(Vec<Self>, Vec<&'a syn::Attribute>)> {
        let attr_iter = iter.into_iter();
        let (attr_len, ..) = attr_iter.size_hint();
        let mut params = Vec::with_capacity(attr_len);
        let mut rest = Vec::new();
        let mut errors = Errors::new();

        for attr in attr_iter {
            match attr.path().get_ident() {
                Some(ident) if ident == "error" => match attr.parse_args_with(Self::parse) {
                    Ok(param) => params.push(param),
                    Err(error) => errors.push(error),
                },
                _ => rest.push(attr),
            }
        }

        errors.finish((params, rest))
    }
}

impl Parse for Param {
    /// Parse one supported `#[error(...)]` parameter form.
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(syn::LitStr) {
            return Ok(Self::lone(ParamKind::Format(input.parse()?)));
        }

        if !lookahead.peek(Ident) {
            return Err(lookahead.error());
        }

        let ident: Ident = input.parse()?;
        let kind = match ident.to_string().as_str() {
            "inline" => {
                let options = if input.peek(Paren) {
                    let content;
                    syn::parenthesized!(content in input);
                    content.parse()?
                } else {
                    Inline::default()
                };
                ParamKind::Inline(options)
            }
            "import" => {
                let content;
                syn::parenthesized!(content in input);
                ParamKind::Import(content.parse()?)
            }
            "source" => {
                let content;
                syn::parenthesized!(content in input);
                ParamKind::Source(content.parse()?)
            }
            "transparent" => {
                let content;
                syn::parenthesized!(content in input);
                ParamKind::Transparent(Transparent(content.parse()?))
            }
            "from" => ParamKind::From,
            "display" => {
                let content;
                syn::parenthesized!(content in input);
                ParamKind::Display(content.parse()?)
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    ident,
                    "expected `inline`, `import`, `source`, `transparent`, `from`, or `display`",
                ));
            }
        };

        Ok(Self::identified(ident, kind))
    }
}

/// Parsed container-wide generation options.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Config {
    /// Requested generated inline policy.
    inline: Option<Inline>,

    /// Requested generated root import.
    import: Option<Import>,
}

impl Config {
    /// Construct parsed container options.
    #[inline]
    #[must_use]
    pub const fn new(inline: Option<Inline>, import: Option<Import>) -> Self {
        Self { inline, import }
    }

    /// Consume the options into their parsed components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Option<Inline>, Option<Import>) {
        let Self { inline, import } = self;

        (inline, import)
    }
}

/// Parsed behavior declarations for one error type or enum variant.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Declaration {
    /// Optional format-string declaration.
    format: Option<Format>,

    /// Optional custom display formatter.
    display: Option<Path>,

    /// Optional ordinary source field reference.
    source: Option<FieldRef>,

    /// Optional transparent field reference.
    transparent: Option<Transparent>,

    /// Whether automatic conversion was requested.
    from: bool,
}

impl Declaration {
    /// Construct one parsed error declaration.
    #[inline]
    #[must_use]
    pub const fn new(
        format: Option<Format>,
        display: Option<Path>,
        source: Option<FieldRef>,
        transparent: Option<Transparent>,
        from: bool,
    ) -> Self {
        Self {
            format,
            display,
            source,
            transparent,
            from,
        }
    }

    /// Consume the declaration into its orthogonal syntax components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Option<Format>, Option<Path>, Option<FieldRef>, Option<Transparent>, bool) {
        let Self {
            format,
            display,
            source,
            transparent,
            from,
        } = self;

        (format, display, source, transparent, from)
    }
}

/// The kind of a parsed error parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamKind {
    /// Generated method inline behavior.
    Inline(Inline),

    /// Root path used by generated implementations.
    Import(Import),

    /// A Rust format string and explicit arguments.
    Format(Format),

    /// An explicit source field.
    Source(FieldRef),

    /// Transparent error behavior over one field.
    Transparent(Transparent),

    /// Automatic conversion from the sole field.
    From,

    /// A custom display formatter path.
    Display(Path),
}

/// The inline policy for generated methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Inline {
    /// Emit ordinary `#[inline]`.
    #[default]
    Neutral,

    /// Emit `#[inline(never)]`.
    Never,

    /// Emit `#[inline(always)]`.
    Always,
}

impl Parse for Inline {
    /// Parse the requested generated-method inline policy.
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

/// The root path used by generated code.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Import(pub Path);

impl Parse for Import {
    /// Parse the root path used by generated trait implementations.
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

/// Transparent behavior over a referenced field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transparent(pub FieldRef);

impl Parse for Transparent {
    /// Parse the field selector used for transparent behavior.
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse().map(Self)
    }
}
