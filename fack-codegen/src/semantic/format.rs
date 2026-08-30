//! Resolved formatting semantics and field requirements.
//!
//! This module resolves source-level captures into owned field proofs and the
//! exact formatting traits and binding policy required by expansion. Once a
//! `Resolved` value exists, expansion performs no field lookup or validation.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use proc_macro2::Span;
use syn::{Ident, LitStr, Token, parse_str, punctuated::Punctuated};

use crate::input::{Argument, Field, Fields, Format};

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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// NOTE(invariant): The field and formatting trait were resolved together from
// one validated format capture.
pub struct FormatUse {
    /// Resolved captured field.
    field: Field,

    /// Formatting trait required by the capture.
    format_trait: FormatTrait,
}

impl FormatUse {
    /// Return the referenced field.
    #[inline]
    #[must_use]
    pub const fn field(&self) -> &Field {
        let &Self { ref field, .. } = self;

        field
    }

    /// Return the formatting trait required by this use.
    #[inline]
    #[must_use]
    pub const fn format_trait(&self) -> FormatTrait {
        let &Self { format_trait, .. } = self;

        format_trait
    }
}

/// A validated format whose field references have been resolved.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Resolved {
    /// A literal that can be written without Rust formatting machinery.
    Static(LitStr),

    /// Dynamic formatting with resolved field and binding requirements.
    Dynamic(DynamicFormat),
}

/// Dynamic formatting after capture resolution.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// NOTE(invariant): The rewritten literal, arguments, uses, and binding policy
// describe one resolved dynamic format.
pub struct DynamicFormat {
    /// Rewritten format string consumed by generated `write!` calls.
    literal: LitStr,

    /// Explicit Rust format arguments supplied by the user.
    arguments: Punctuated<Argument, Token![,]>,

    /// Formatting requirements introduced by captured fields.
    uses: Vec<FormatUse>,

    /// Fields that generated code must bind before formatting.
    bindings: BindingRequirement,
}

/// Field binding policy established during format resolution.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BindingRequirement {
    /// Bind only fields proven necessary by format captures.
    Selected(SelectedFields),

    /// Bind every declared field because explicit expressions may reference any
    /// field.
    All,
}

/// Unique selected fields in first use order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
// NOTE(invariant): Every field appears at most once and stored order matches
// first capture use.
pub struct SelectedFields(Vec<Field>);

/// Semantic class of one format capture head.
enum CaptureHead<'text> {
    /// Empty capture resolved by Rust formatting itself.
    Empty,

    /// Positional tuple field capture.
    Positional(&'text str),

    /// Name owned by an explicit format argument.
    Explicit,

    /// Candidate named field capture.
    Named(&'text str),
}

impl<'text> CaptureHead<'text> {
    /// Classify a capture head before field resolution.
    fn classify(argument: &'text str, explicit_names: &[String]) -> Self {
        let empty = argument.is_empty();
        let positional = argument.as_bytes().iter().all(u8::is_ascii_digit);
        let explicit = explicit_names.iter().any(|name| name == argument);

        match (empty, positional, explicit) {
            (true, _, _) => Self::Empty,
            (false, true, _) => Self::Positional(argument),
            (false, false, true) => Self::Explicit,
            (false, false, false) => Self::Named(argument),
        }
    }
}

/// Semantic class of one dynamic width or precision reference.
enum DynamicReference<'text> {
    /// Positional tuple field reference.
    Positional(&'text str),

    /// Candidate named field reference.
    Named(&'text str),
}

impl<'text> DynamicReference<'text> {
    /// Classify one dynamic format reference.
    fn classify(token: &'text str) -> Self {
        if token.as_bytes().iter().all(u8::is_ascii_digit) {
            Self::Positional(token)
        } else {
            Self::Named(token)
        }
    }
}

impl SelectedFields {
    /// Record one field while preserving uniqueness and first use order.
    fn push(&mut self, field: Field) {
        let &mut Self(ref mut fields) = self;

        if !fields.contains(&field) {
            fields.push(field);
        }
    }

    /// Return selected identities in first use order.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[Field] {
        let &Self(ref fields) = self;

        fields
    }
}

impl Resolved {
    /// Resolve every field dependent part of one source format.
    #[inline]
    pub fn resolve(raw: Format, fields: &Fields) -> syn::Result<Self> {
        let (literal, arguments) = raw.parts();

        let explicit_names = Self::explicit_names(&arguments);
        let has_positional = arguments.iter().any(|argument| argument.name().is_none());
        let value = literal.value();
        let mut read = value.as_str();
        let mut output = String::with_capacity(value.len());
        let mut uses = Vec::new();
        let mut selected = SelectedFields::default();

        while let Some(open) = read.find('{') {
            let (prefix, rest) = read.split_at(open);

            output.push_str(prefix);
            read = rest;

            if let Some(rest) = read.strip_prefix("{{") {
                output.push_str("{{");
                read = rest;
                continue;
            }

            let (_, after_open) = read.split_at(1);

            let close = after_open.find('}');

            match close {
                Some(close) => {
                    let (inside, suffix) = after_open.split_at(close);

                    let (_, rest) = suffix.split_at(1);

                    let rewritten = Self::resolve_capture(
                        inside,
                        fields,
                        &explicit_names,
                        has_positional,
                        &mut uses,
                        &mut selected,
                        literal.span(),
                    )?;

                    output.push('{');
                    output.push_str(&rewritten);
                    output.push('}');
                    read = rest;
                }
                None => {
                    output.push_str(read);
                    read = "";
                    break;
                }
            }
        }

        output.push_str(read);

        let static_message = arguments.is_empty() && !value.contains('{') && !value.contains('}');
        let literal = LitStr::new(&output, literal.span());

        match static_message {
            true => Ok(Self::Static(literal)),
            false => {
                let bindings = match arguments.is_empty() {
                    true => BindingRequirement::Selected(selected),
                    false => BindingRequirement::All,
                };
                let format = DynamicFormat {
                    literal,
                    arguments,
                    uses,
                    bindings,
                };

                Ok(Self::Dynamic(format))
            }
        }
    }

    /// Resolve one `{...}` capture and rewrite tuple fields to generated local
    /// names.
    fn resolve_capture(
        inside: &str,
        fields: &Fields,
        explicit_names: &[String],
        has_positional: bool,
        uses: &mut Vec<FormatUse>,
        selected: &mut SelectedFields,
        span: Span,
    ) -> syn::Result<String> {
        let argument_end = inside.find(':').unwrap_or(inside.len());
        let argument = &inside[..argument_end];
        let spec = &inside[argument_end..];
        let mut rewritten = String::with_capacity(inside.len() + 1);

        let field = Self::capture_field(argument, fields, explicit_names, span)?;
        let ambiguous = argument.as_bytes().first().is_some_and(u8::is_ascii_digit) && has_positional;

        match (field, ambiguous) {
            (Some(_), true) => Err(syn::Error::new(
                span,
                "ambiguous numeric field capture with explicit positional format arguments",
            )),
            (Some(field), false) => {
                selected.push(field.clone());
                uses.push(FormatUse {
                    field: field.clone(),
                    format_trait: Self::format_trait(spec),
                });

                match field.name() {
                    Some(_) => rewritten.push_str(argument),
                    None => {
                        rewritten.push('_');
                        rewritten.push_str(argument);
                    }
                }

                Ok(())
            }
            (None, _) => {
                rewritten.push_str(argument);

                Ok(())
            }
        }?;

        rewritten.push_str(&Self::resolve_dynamic_spec(spec, fields, selected, span)?);

        Ok(rewritten)
    }

    /// Interpret a capture head as a field only when explicit arguments do not
    /// own it.
    fn capture_field(argument: &str, fields: &Fields, explicit_names: &[String], span: Span) -> syn::Result<Option<Field>> {
        match CaptureHead::classify(argument, explicit_names) {
            CaptureHead::Empty | CaptureHead::Explicit => Ok(None),
            CaptureHead::Positional(argument) => {
                let index = argument
                    .parse::<usize>()
                    .map_err(|_error| syn::Error::new(span, "invalid positional format capture"))?;

                Ok(fields.tuple_capture(index))
            }
            CaptureHead::Named(argument) => {
                let mut ident = parse_str::<Ident>(argument).map_err(|_error| syn::Error::new(span, "invalid named format capture"))?;

                ident.set_span(span);

                fields.named(&ident).map(Some)
            }
        }
    }

    /// Resolve field references used by dynamic width and precision specifiers.
    fn resolve_dynamic_spec(spec: &str, fields: &Fields, selected: &mut SelectedFields, span: Span) -> syn::Result<String> {
        let mut output = String::with_capacity(spec.len());
        let bytes = spec.as_bytes();
        let mut index = 0;

        while let Some(byte) = bytes.get(index).copied() {
            let start = index;
            let is_word = byte.is_ascii_alphanumeric() || byte == b'_';

            if !is_word {
                output.push(char::from(byte));
                index += 1;
                continue;
            }

            while bytes.get(index).is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_') {
                index += 1;
            }

            let token = spec
                .get(start..index)
                .ok_or_else(|| syn::Error::new(span, "invalid dynamic format specifier"))?;
            let dynamic = bytes.get(index) == Some(&b'$');

            if !dynamic {
                output.push_str(token);
                continue;
            }

            match DynamicReference::classify(token) {
                DynamicReference::Positional(token) => {
                    let field_index = token
                        .parse::<usize>()
                        .map_err(|_error| syn::Error::new(span, "invalid dynamic format field index"))?;

                    match fields.tuple_capture(field_index) {
                        Some(field) => {
                            selected.push(field);
                            output.push('_');
                        }
                        None => {}
                    }
                }
                DynamicReference::Named(token) => {
                    let field = parse_str::<Ident>(token).ok().and_then(|ident| fields.named(&ident).ok());

                    match field {
                        Some(field) => selected.push(field),
                        None => {}
                    }
                }
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
    fn explicit_names(arguments: &Punctuated<Argument, Token![,]>) -> Vec<String> {
        arguments.iter().filter_map(Argument::name).map(ToString::to_string).collect()
    }
}

impl Resolved {
    /// Return formatting requirements introduced by captured fields.
    #[inline]
    #[must_use]
    pub fn uses(&self) -> &[FormatUse] {
        match self {
            &Self::Static(_) => &[],
            &Self::Dynamic(ref format) => format.uses(),
        }
    }
}

impl DynamicFormat {
    /// Return the rewritten format literal consumed by generated code.
    #[inline]
    #[must_use]
    pub const fn literal(&self) -> &LitStr {
        let &Self { ref literal, .. } = self;

        literal
    }

    /// Return explicit Rust format arguments.
    #[inline]
    #[must_use]
    pub const fn arguments(&self) -> &Punctuated<Argument, Token![,]> {
        let &Self { ref arguments, .. } = self;

        arguments
    }

    /// Return formatting requirements introduced by captures.
    #[inline]
    #[must_use]
    pub fn uses(&self) -> &[FormatUse] {
        let &Self { ref uses, .. } = self;

        uses
    }

    /// Return the field binding policy established during resolution.
    #[inline]
    #[must_use]
    pub const fn bindings(&self) -> &BindingRequirement {
        let &Self { ref bindings, .. } = self;

        bindings
    }
}
