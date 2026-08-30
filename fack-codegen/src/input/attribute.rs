//! Parsed `#[error(...)]` syntax that is independent of semantic validation.
//!
//! This module owns the helper-attribute grammar and the first strong state
//! transition from raw parameters into coherent display, source, conversion,
//! import, and inline policies.

use alloc::{string::ToString, vec::Vec};

use proc_macro2::Span;
use syn::{
    Ident, Path,
    parse::{Parse, ParseStream},
    token::Paren,
};

use super::{diagnostic::Errors, field::FieldRef, format::Format};

/// A parsed `#[error(...)]` parameter.
#[derive(Clone, Debug)]
// NOTE(invariant): The span, optional identifier, and parameter kind are parsed from the same helper attribute parameter.
pub struct Param {
    /// Source span that introduced the parameter.
    span: Span,

    /// Optional identifier that introduced the parameter.
    name: Option<Ident>,

    /// Parsed parameter value.
    kind: ParamKind,
}

impl Param {
    /// Construct an unnamed parameter.
    #[inline]
    #[must_use]
    pub const fn lone(span: Span, kind: ParamKind) -> Self {
        Self { span, name: None, kind }
    }

    /// Construct a named parameter.
    #[inline]
    #[must_use]
    pub const fn identified(span: Span, name: Ident, kind: ParamKind) -> Self {
        Self {
            span,
            name: Some(name),
            kind,
        }
    }

    /// Return the source span that introduced this parameter.
    #[inline]
    #[must_use]
    pub const fn span(&self) -> Span {
        let &Self { span, .. } = self;

        span
    }

    /// Consume the parameter into its optional introducer and parsed value.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Option<Ident>, ParamKind) {
        let Self { name, kind, .. } = self;

        (name, kind)
    }

    /// Parse every `#[error(...)]` attribute in source order.
    #[inline]
    pub fn classify<'a>(iter: impl IntoIterator<Item = &'a syn::Attribute>) -> syn::Result<Vec<Self>> {
        let attr_iter = iter.into_iter();

        let (attr_len, ..) = attr_iter.size_hint();

        let mut params = Vec::with_capacity(attr_len);
        let mut errors = Errors::new();

        for attr in attr_iter {
            if attr.path().get_ident().is_some_and(|ident| ident == "error") {
                match attr.parse_args_with(Self::parse) {
                    Ok(param) => params.push(param),
                    Err(error) => errors.push(error),
                }
            }
        }

        errors.finish(params)
    }
}

impl Parse for Param {
    /// Parse one supported `#[error(...)]` parameter form.
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        let kind = (lookahead.peek(syn::LitStr), lookahead.peek(Ident));

        match kind {
            (true, _) => {
                let format: Format = input.parse()?;
                let span = format.span();

                Ok(Self::lone(span, ParamKind::Format(format)))
            }
            (false, false) => Err(lookahead.error()),
            (false, true) => {
                let ident: Ident = input.parse()?;
                let kind = match ident.to_string().as_str() {
                    "inline" => {
                        let options = if input.peek(Paren) {
                            let content;
                            syn::parenthesized!(content in input);
                            content.parse()?
                        } else {
                            InlinePolicy::Neutral
                        };

                        Ok(ParamKind::Inline(options))
                    }
                    "import" => {
                        let content;
                        syn::parenthesized!(content in input);
                        Ok(ParamKind::Import(content.parse()?))
                    }
                    "source" => {
                        let content;
                        syn::parenthesized!(content in input);
                        Ok(ParamKind::Source(content.parse()?))
                    }
                    "transparent" => {
                        let content;
                        syn::parenthesized!(content in input);
                        Ok(ParamKind::Transparent(Transparent(content.parse()?)))
                    }
                    "from" => Ok(ParamKind::From(ident.clone())),
                    "display" => {
                        let content;
                        syn::parenthesized!(content in input);
                        Ok(ParamKind::Display(content.parse()?))
                    }
                    _ => Err(syn::Error::new_spanned(
                        &ident,
                        "expected `inline`, `import`, `source`, `transparent`, `from`, or `display`",
                    )),
                }?;

                let span = ident.span();

                Ok(Self::identified(span, ident, kind))
            }
        }
    }
}

/// Parsed container wide generation options.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
// NOTE(invariant): Inline and import policies are each singular after duplicate classification.
pub struct Config {
    /// Requested generated inline policy.
    inline: InlinePolicy,

    /// Requested generated root import.
    import: ImportRoot,
}

impl Config {
    /// Construct parsed container options from raw occurrences.
    #[inline]
    #[must_use]
    pub fn new(inline: Option<InlinePolicy>, import: Option<Import>) -> Self {
        let inline = inline.unwrap_or_default();
        let import = match import {
            Some(Import(path)) => ImportRoot::Explicit(path),
            None => ImportRoot::Core,
        };

        Self { inline, import }
    }

    /// Consume the options into their semantic components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (InlinePolicy, ImportRoot) {
        let Self { inline, import } = self;

        (inline, import)
    }
}

/// Parsed behavior for one syntactically coherent error declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Declaration {
    /// Ordinary display with optional source and conversion behavior.
    Ordinary(Ordinary),

    /// Transparent behavior through one selected field.
    Transparent(FieldRef),
}

impl Declaration {
    /// Classify raw declaration occurrences into one coherent syntax state.
    #[inline]
    pub fn classify(
        subject: &Ident,
        format: Option<(proc_macro2::Span, Format)>,
        display: Option<(proc_macro2::Span, Path)>,
        source: Option<(proc_macro2::Span, FieldRef)>,
        transparent: Option<(proc_macro2::Span, Transparent)>,
        from: Option<(proc_macro2::Span, Ident)>,
    ) -> syn::Result<Self> {
        let transparent = transparent.map(|(span, Transparent(field))| (span, field));

        match (transparent, format, display, source, from) {
            (Some((_, field)), None, None, None, None) => Ok(Self::Transparent(field)),
            (Some(_), Some((span, _)), _, _, _) | (Some(_), _, Some((span, _)), _, _) => {
                Err(syn::Error::new(span, "transparent error cannot also declare display formatting"))
            }
            (Some(_), None, None, Some((span, _)), _) => {
                Err(syn::Error::new(span, "transparent error cannot also declare an ordinary source"))
            }
            (Some(_), None, None, None, Some((span, _))) => Err(syn::Error::new(span, "transparent error cannot also derive `From`")),
            (None, Some(_), Some((span, _)), _, _) => Err(syn::Error::new(
                span,
                "error cannot declare both a format string and `display(...)`",
            )),
            (None, None, None, _, _) => Err(syn::Error::new_spanned(
                subject,
                "error requires a format string, `display(...)`, or `transparent(...)`",
            )),
            (None, Some((_, format)), None, source, from) => {
                let display = Display::Format(format);
                let source = Source::classify(source.map(|(_, field)| field), from.map(|(_, from)| from));

                Ok(Self::Ordinary(Ordinary { display, source }))
            }
            (None, None, Some((_, path)), source, from) => {
                let display = Display::Custom(path);
                let source = Source::classify(source.map(|(_, field)| field), from.map(|(_, from)| from));

                Ok(Self::Ordinary(Ordinary { display, source }))
            }
        }
    }
}

/// One ordinary error declaration after syntax classification.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// NOTE(invariant): Display and source states are syntactically coherent before this value is constructed.
pub struct Ordinary {
    /// Selected ordinary display behavior.
    display: Display,

    /// Selected ordinary source and conversion behavior.
    source: Source,
}

impl Ordinary {
    /// Consume the ordinary declaration into its syntax components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Display, Source) {
        let Self { display, source } = self;

        (display, source)
    }
}

/// Ordinary display syntax.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Display {
    /// A Rust format string and explicit arguments.
    Format(Format),

    /// A custom formatter function.
    Custom(Path),
}

/// Ordinary source and conversion syntax.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// No source and no automatic conversion.
    None,

    /// One explicit ordinary source field.
    Explicit(FieldRef),

    /// Automatic conversion from the sole field.
    From(Ident),

    /// Automatic conversion with an explicit source selector.
    ExplicitFrom {
        /// Explicit source selector.
        field: FieldRef,

        /// Source `from` keyword that requested conversion.
        from: Ident,
    },
}

impl Source {
    /// Classify source and conversion occurrences into one syntax state.
    #[inline]
    #[must_use]
    fn classify(source: Option<FieldRef>, from: Option<Ident>) -> Self {
        match (source, from) {
            (None, None) => Self::None,
            (Some(field), None) => Self::Explicit(field),
            (None, Some(from)) => Self::From(from),
            (Some(field), Some(from)) => Self::ExplicitFrom { field, from },
        }
    }
}

/// The kind of a parsed error parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamKind {
    /// Generated method inline behavior.
    Inline(InlinePolicy),

    /// Root path used by generated implementations.
    Import(Import),

    /// A Rust format string and explicit arguments.
    Format(Format),

    /// An explicit source field.
    Source(FieldRef),

    /// Transparent error behavior over one field.
    Transparent(Transparent),

    /// Automatic conversion from the sole field.
    From(Ident),

    /// A custom display formatter path.
    Display(Path),
}

/// The inline policy for generated methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InlinePolicy {
    /// Emit no explicit inline annotation.
    #[default]
    Unspecified,

    /// Emit ordinary `#[inline]`.
    Neutral,

    /// Emit `#[inline(never)]`.
    Never,

    /// Emit `#[inline(always)]`.
    Always,
}

impl Parse for InlinePolicy {
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

/// The generated root path policy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImportRoot {
    /// Generate paths through `::core`.
    #[default]
    Core,

    /// Generate paths through an explicitly selected root.
    Explicit(Path),
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
