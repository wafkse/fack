//! Proof transition from parsed declarations to validated semantics.

use alloc::{boxed::Box, vec::Vec};

use proc_macro2::TokenStream;
use syn::{Ident, Path};

use crate::{
    enumerate,
    field::{FieldRef, Fields},
    format::Format,
    resolve::Resolve as _,
    semantics::{
        Conversion, Display, Enumeration as ValidEnumeration, ErrorSource, Header, Structure as ValidStructure, Target as ValidTarget,
        Variant as ValidVariant,
    },
    source::{Source, SourceShape},
    structure,
    syntax::Transparent,
    target::Kind,
};

/// Proof that an internal value passed its [`Validate`] transition.
// NOTE(invariant): The wrapped value can only be created by this module after
// the corresponding validation routine succeeds. Consumers may rely on the
// semantic checks performed by that transition.
#[derive(Clone, Debug)]
struct Validated<ValueType>(ValueType);

impl<ValueType> Validated<ValueType> {
    /// Wrap a value only at a validation boundary owned by this module.
    const fn new(value: ValueType) -> Self {
        Self(value)
    }

    /// Consume the proof and return the validated value.
    #[inline]
    #[must_use]
    fn into_inner(self) -> ValueType {
        let Self(value) = self;

        value
    }
}

/// Validate a parsed representation into a stronger semantic representation.
trait Validate {
    /// Stronger representation produced after validation succeeds.
    type Output;

    /// Validate the parsed value.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when declarations conflict or field references do
    /// not satisfy their semantic requirements.
    fn validate(self) -> syn::Result<Self::Output>;
}

/// A validated derive target that can be expanded safely.
// NOTE(invariant): Construction succeeds only after the complete parsed target
// has passed semantic validation. Expansion may rely on those checks without
// repeating them.
#[derive(Clone, Debug)]
pub struct ValidatedTarget(ValidTarget);

impl ValidatedTarget {
    /// Expand the validated target into Rust implementation tokens.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if final token construction discovers an invalid
    /// generated Rust fragment.
    #[inline]
    pub fn expand(self) -> syn::Result<TokenStream> {
        let Self(target) = self;

        crate::expand::target(target)
    }
}

/// Validate one parsed target kind and establish the public proof state.
pub fn target(target: Kind) -> syn::Result<ValidatedTarget> {
    let target = match target {
        Kind::Struct(structure) => {
            let structure = structure.validate()?.into_inner();

            ValidTarget::Struct(Box::new(structure))
        }
        Kind::Enum(enumeration) => {
            let enumeration = enumeration.validate()?.into_inner();

            ValidTarget::Enum(enumeration)
        }
    };

    Ok(ValidatedTarget(target))
}

impl Validate for structure::Structure {
    /// Proof-bearing structure semantics produced after declaration checks.
    type Output = Validated<ValidStructure>;

    /// Resolve one parsed structure into validated display, source, and
    /// conversion behavior.
    #[inline]
    fn validate(self) -> syn::Result<Self::Output> {
        let (config, name, generics, fields, declaration) = self.parts();
        let (inline, import) = config.parts();
        let (format, display, source, transparent, from) = declaration.parts();
        let declaration = Declaration {
            subject: name.clone(),
            fields: &fields,
            format,
            display,
            source,
            transparent,
            from,
        };
        let (display, source, conversion) = declaration.validate()?;
        let header = Header::new(inline, import, name, generics);

        let structure = ValidStructure::new(header, fields, display, source, conversion);

        Ok(Validated::new(structure))
    }
}

impl Validate for enumerate::Enumeration {
    /// Proof-bearing enumeration semantics with every variant already
    /// validated.
    type Output = Validated<ValidEnumeration>;

    /// Validate every variant before constructing the enumeration proof.
    #[inline]
    fn validate(self) -> syn::Result<Self::Output> {
        let (config, name, generics, variants) = self.parts();
        let (inline, import) = config.parts();
        let mut validated = Vec::with_capacity(variants.len());

        for variant in variants {
            validated.push(variant.validate()?.into_inner());
        }

        let header = Header::new(inline, import, name, generics);

        let enumeration = ValidEnumeration::new(header, validated);

        Ok(Validated::new(enumeration))
    }
}

impl Validate for enumerate::Variant {
    /// Proof-bearing variant semantics produced after declaration checks.
    type Output = Validated<ValidVariant>;

    /// Resolve one parsed variant into validated display, source, and
    /// conversion behavior.
    #[inline]
    fn validate(self) -> syn::Result<Self::Output> {
        let (name, fields, declaration) = self.parts();
        let (format, display, source, transparent, from) = declaration.parts();
        let declaration = Declaration {
            subject: name.clone(),
            fields: &fields,
            format,
            display,
            source,
            transparent,
            from,
        };
        let (display, source, conversion) = declaration.validate()?;

        let variant = ValidVariant::new(name, fields, display, source, conversion);

        Ok(Validated::new(variant))
    }
}

/// One set of declarations whose semantic compatibility must be proven.
struct Declaration<'fields> {
    /// Syntax identifier used as the diagnostic subject.
    subject: Ident,

    /// Fields against which references are resolved.
    fields: &'fields Fields,

    /// Optional parsed format declaration.
    format: Option<Format>,

    /// Optional custom display formatter.
    display: Option<Path>,

    /// Optional ordinary source reference.
    source: Option<FieldRef>,

    /// Optional transparent source reference.
    transparent: Option<Transparent>,

    /// Whether automatic conversion was requested.
    from: bool,
}

impl Declaration<'_> {
    /// Check declaration compatibility and resolve field-dependent semantics
    /// once.
    fn validate(self) -> syn::Result<(Display, ErrorSource, Option<Conversion>)> {
        let Self {
            subject,
            fields,
            format,
            display,
            source,
            transparent,
            from,
        } = self;

        if format.is_some() && display.is_some() {
            return Err(syn::Error::new_spanned(
                subject,
                "error cannot declare both a format string and `display(...)`",
            ));
        }

        if transparent.is_some() && (format.is_some() || display.is_some()) {
            return Err(syn::Error::new_spanned(
                subject,
                "transparent error cannot also declare display formatting",
            ));
        }

        if transparent.is_some() && source.is_some() {
            return Err(syn::Error::new_spanned(
                subject,
                "transparent error cannot also declare an ordinary source",
            ));
        }

        if transparent.is_some() && from {
            return Err(syn::Error::new_spanned(subject, "transparent error cannot also derive `From`"));
        }

        if let Some(Transparent(field_ref)) = transparent {
            let field = field_ref.resolve(fields)?;
            let sole = fields.sole()?;

            if field != sole {
                return Err(syn::Error::new_spanned(subject, "transparent error must target its sole field"));
            }

            let source = Source::new(fields, field);

            if matches!(source.shape(), SourceShape::Optional | SourceShape::OptionalBoxed) {
                return Err(syn::Error::new_spanned(subject, "transparent error source cannot be optional"));
            }

            let display = Display::Transparent(field);
            let source = ErrorSource::Transparent(source);

            return Ok((display, source, None));
        }

        let display = match (format, display) {
            (Some(format), None) => Display::Format(format.resolve(fields)?),
            (None, Some(path)) => Display::Custom(path),
            (None, None) => {
                return Err(syn::Error::new_spanned(
                    subject,
                    "error requires a format string, `display(...)`, or `transparent(...)`",
                ));
            }
            (Some(_), Some(_)) => {
                return Err(syn::Error::new_spanned(subject, "error has conflicting display declarations"));
            }
        };

        let conversion = if from {
            let field = fields.sole()?;
            let field_type = fields.ty(field).clone();

            Some(Conversion::new(field, field_type))
        } else {
            None
        };

        let source = match (source, conversion.as_ref()) {
            (Some(field_ref), Some(conversion)) => {
                let field = field_ref.resolve(fields)?;

                if field != conversion.field() {
                    return Err(syn::Error::new_spanned(
                        subject,
                        "`from` conversion and explicit source must refer to the same field",
                    ));
                }

                ErrorSource::Field(Source::new(fields, field))
            }
            (Some(field_ref), None) => {
                let field = field_ref.resolve(fields)?;

                ErrorSource::Field(Source::new(fields, field))
            }
            (None, Some(conversion)) => ErrorSource::Field(Source::new(fields, conversion.field())),
            (None, None) => ErrorSource::None,
        };

        Ok((display, source, conversion))
    }
}
