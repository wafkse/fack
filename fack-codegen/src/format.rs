//! Format syntax, field capture resolution, and formatting requirements.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Ident, LitStr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::{
    expand::Expand,
    field::{FieldId, Fields},
    resolve::Resolve,
};

/// A source-level Rust format string and explicit arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
            let _ = input.parse::<Token![,]>()?;

            Punctuated::<Argument, Token![,]>::parse_terminated(input)?
        } else {
            Punctuated::new()
        };

        Ok(Self { literal, arguments })
    }
}

/// One explicit Rust format argument.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
            let _ = input.parse::<Token![=]>()?;

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
        let Self { name, expression } = self;

        match name {
            Some(name) => quote::quote!(#name = #expression).to_tokens(tokens),
            None => expression.to_tokens(tokens),
        }
    }
}

/// A formatting trait required by one captured field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormatTrait {
    /// `Display` formatting.
    Display,

    /// `Debug` formatting.
    Debug,

    /// Lower hexadecimal formatting.
    LowerHex,

    /// Upper hexadecimal formatting.
    UpperHex,

    /// Octal formatting.
    Octal,

    /// Binary formatting.
    Binary,

    /// Lower exponential formatting.
    LowerExp,

    /// Upper exponential formatting.
    UpperExp,

    /// Pointer formatting.
    Pointer,
}

/// One resolved field use in a format string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormatUse {
    /// Resolved captured field.
    field: FieldId,

    /// Formatting trait required by the capture.
    format_trait: FormatTrait,
}

impl FormatUse {
    /// Return the referenced field.
    #[inline]
    #[must_use]
    pub const fn field(&self) -> FieldId {
        let Self { field, .. } = self;

        *field
    }

    /// Return the formatting trait required by this use.
    #[inline]
    #[must_use]
    pub const fn format_trait(&self) -> FormatTrait {
        let Self { format_trait, .. } = self;

        *format_trait
    }
}

/// A validated format string whose field references have been resolved.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Resolved {
    /// The rewritten format string consumed by generated `write!` calls.
    literal: LitStr,

    /// Explicit Rust format arguments supplied by the user.
    arguments: syn::punctuated::Punctuated<Argument, syn::Token![,]>,

    /// Formatting requirements introduced by captured fields.
    uses: Vec<FormatUse>,

    /// Every field that must be bound for formatting.
    fields: Vec<FieldId>,

    /// Whether the message can use `Formatter::write_str` directly.
    static_message: bool,
}

impl Resolved {
    /// Resolve format captures against one field list.
    fn resolve_format(raw: Format, fields: &Fields) -> syn::Result<Self> {
        let explicit_names = Self::explicit_names(&raw.arguments);
        let has_positional = raw.arguments.iter().any(|argument| Self::explicit_name(argument).is_none());
        let value = raw.literal.value();
        let mut read = value.as_str();
        let mut output = String::with_capacity(value.len());
        let mut uses = Vec::new();
        let mut selected = Vec::new();

        while let Some(open) = read.find('{') {
            output.push_str(&read[..open]);
            read = &read[open..];

            if let Some(rest) = read.strip_prefix("{{") {
                output.push_str("{{");
                read = rest;
                continue;
            }

            let Some(close) = read[1..].find('}') else {
                output.push_str(read);
                read = "";
                break;
            };
            let close = close + 1;
            let inside = &read[1..close];
            let rewritten = Self::resolve_capture(
                inside,
                fields,
                &explicit_names,
                has_positional,
                &mut uses,
                &mut selected,
                raw.literal.span(),
            )?;

            output.push('{');
            output.push_str(&rewritten);
            output.push('}');
            read = &read[close + 1..];
        }

        output.push_str(read);

        let static_message = raw.arguments.is_empty() && !value.contains('{') && !value.contains('}');
        let format = LitStr::new(&output, raw.literal.span());

        Ok(Self {
            literal: format,
            arguments: raw.arguments,
            uses,
            fields: selected,
            static_message,
        })
    }

    /// Resolve one `{...}` capture and rewrite tuple fields to generated local
    /// names.
    fn resolve_capture(
        inside: &str,
        fields: &Fields,
        explicit_names: &[String],
        has_positional: bool,
        uses: &mut Vec<FormatUse>,
        selected: &mut Vec<FieldId>,
        span: proc_macro2::Span,
    ) -> syn::Result<String> {
        let argument_end = inside.find(':').unwrap_or(inside.len());
        let argument = &inside[..argument_end];
        let spec = &inside[argument_end..];
        let mut rewritten = String::with_capacity(inside.len() + 1);

        match Self::capture_field(argument, fields, explicit_names, span)? {
            Some(field) => {
                if argument.as_bytes().first().is_some_and(u8::is_ascii_digit) && has_positional {
                    return Err(syn::Error::new(
                        span,
                        "ambiguous numeric field capture with explicit positional format arguments",
                    ));
                }

                Self::push_field(selected, field);
                uses.push(FormatUse {
                    field,
                    format_trait: Self::format_trait(spec),
                });

                match fields.name(field) {
                    Some(_) => rewritten.push_str(argument),
                    None => {
                        rewritten.push('_');
                        rewritten.push_str(argument);
                    }
                }
            }
            None => rewritten.push_str(argument),
        }

        rewritten.push_str(&Self::resolve_dynamic_spec(spec, fields, selected, span)?);

        Ok(rewritten)
    }

    /// Interpret a capture head as a field only when explicit arguments do not
    /// own it.
    fn capture_field(argument: &str, fields: &Fields, explicit_names: &[String], span: proc_macro2::Span) -> syn::Result<Option<FieldId>> {
        if argument.is_empty() {
            return Ok(None);
        }

        if argument.as_bytes().iter().all(u8::is_ascii_digit) {
            let index = argument
                .parse::<usize>()
                .map_err(|_| syn::Error::new(span, "invalid positional format capture"))?;

            return match index < fields.len() && fields.name(FieldId::from_index(index)).is_none() {
                true => Ok(Some(FieldId::from_index(index))),
                false => Ok(None),
            };
        }

        if explicit_names.iter().any(|name| name == argument) {
            return Ok(None);
        }

        let ident = syn::parse_str::<syn::Ident>(argument).map_err(|_| syn::Error::new(span, "invalid named format capture"))?;
        fields.named(&ident).map(Some)
    }

    /// Resolve field references used by dynamic width and precision specifiers.
    fn resolve_dynamic_spec(spec: &str, fields: &Fields, selected: &mut Vec<FieldId>, span: proc_macro2::Span) -> syn::Result<String> {
        let mut output = String::with_capacity(spec.len());
        let bytes = spec.as_bytes();
        let mut index = 0;

        while index < bytes.len() {
            let start = index;
            let is_word = bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_';

            if !is_word {
                output.push(bytes[index] as char);
                index += 1;
                continue;
            }

            while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_') {
                index += 1;
            }

            let token = &spec[start..index];
            let dynamic = bytes.get(index) == Some(&b'$');

            if !dynamic {
                output.push_str(token);
                continue;
            }

            if token.as_bytes().iter().all(u8::is_ascii_digit) {
                let field_index = token
                    .parse::<usize>()
                    .map_err(|_| syn::Error::new(span, "invalid dynamic format field index"))?;

                if field_index < fields.len() && fields.name(FieldId::from_index(field_index)).is_none() {
                    let field = FieldId::from_index(field_index);
                    Self::push_field(selected, field);
                    output.push('_');
                }
            } else if let Ok(ident) = syn::parse_str::<syn::Ident>(token)
                && let Ok(field) = fields.named(&ident)
            {
                Self::push_field(selected, field);
            }

            output.push_str(token);
        }

        Ok(output)
    }

    /// Classify the Rust formatting trait required by one format specifier.
    fn format_trait(spec: &str) -> FormatTrait {
        match spec.chars().next_back() {
            Some('?') => FormatTrait::Debug,
            Some('o') => FormatTrait::Octal,
            Some('x') => FormatTrait::LowerHex,
            Some('X') => FormatTrait::UpperHex,
            Some('p') => FormatTrait::Pointer,
            Some('b') => FormatTrait::Binary,
            Some('e') => FormatTrait::LowerExp,
            Some('E') => FormatTrait::UpperExp,
            _ => FormatTrait::Display,
        }
    }

    /// Collect names reserved by explicit user-supplied format arguments.
    fn explicit_names(arguments: &syn::punctuated::Punctuated<Argument, syn::Token![,]>) -> Vec<String> {
        arguments
            .iter()
            .filter_map(|argument| argument.name.as_ref())
            .map(ToString::to_string)
            .collect()
    }

    /// Return the explicit name of one format argument when present.
    const fn explicit_name(argument: &Argument) -> Option<&syn::Ident> {
        argument.name.as_ref()
    }

    /// Record a selected field once while preserving first-use order.
    fn push_field(fields: &mut Vec<FieldId>, field: FieldId) {
        if !fields.contains(&field) {
            fields.push(field);
        }
    }
}

impl Resolved {
    /// Return the rewritten format literal consumed by generated code.
    #[inline]
    #[must_use]
    pub const fn literal(&self) -> &LitStr {
        let Self { literal, .. } = self;

        literal
    }

    /// Return explicit Rust format arguments.
    #[inline]
    #[must_use]
    pub const fn arguments(&self) -> &Punctuated<Argument, Token![,]> {
        let Self { arguments, .. } = self;

        arguments
    }

    /// Return formatting requirements introduced by captured fields.
    #[inline]
    #[must_use]
    pub fn uses(&self) -> &[FormatUse] {
        let Self { uses, .. } = self;

        uses
    }

    /// Return every field required by formatting.
    #[inline]
    #[must_use]
    pub fn fields(&self) -> &[FieldId] {
        let Self { fields, .. } = self;

        fields
    }

    /// Return whether formatting can use `Formatter::write_str` directly.
    #[inline]
    #[must_use]
    pub const fn is_static(&self) -> bool {
        let Self { static_message, .. } = self;

        *static_message
    }
}

impl Resolve for Format {
    /// Field collection used to resolve captures.
    type Context = Fields;

    /// Format representation whose field references are already validated.
    type Output = Resolved;

    /// Resolve every field-dependent part of the source format.
    #[inline]
    fn resolve(self, fields: &Self::Context) -> syn::Result<Self::Output> {
        Resolved::resolve_format(self, fields)
    }
}

/// Context required to expand one resolved format body.
#[derive(Clone, Debug)]
pub struct ExpandContext(TokenStream);

impl ExpandContext {
    /// Construct a format expansion context.
    #[inline]
    #[must_use]
    pub const fn new(root: TokenStream) -> Self {
        Self(root)
    }
}

impl Expand for &Resolved {
    /// Root-path context needed by non-static `write!` expansion.
    type Context = ExpandContext;

    /// Generate either `write_str` or `write!` from the resolved
    /// representation.
    #[inline]
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let ExpandContext(root) = context;
        let literal = self.literal();
        let arguments = self.arguments();

        if self.is_static() {
            Ok(quote! { f.write_str(#literal) })
        } else {
            Ok(quote! { #root::write!(f, #literal, #arguments) })
        }
    }
}
