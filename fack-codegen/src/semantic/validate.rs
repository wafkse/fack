//! Proof transition from parsed declarations to validated semantics.
//!
//! Validation resolves every field selector and format capture exactly once,
//! then constructs semantic states that expansion can consume without defensive
//! checks or source-level lookup.

use alloc::{boxed::Box, vec::Vec};

use syn::spanned::Spanned;

use crate::input::{
    Declaration as InputDeclaration, Display as InputDisplay, Enumeration as InputEnumeration, FieldRef, Fields, Source as InputSource,
    Structure as InputStructure, Target as InputTarget, Variant as InputVariant,
};

use super::{
    Conversion, Display, Enumeration as ValidEnumeration, ErrorSource, Header, Resolved, Structure as ValidStructure,
    Target as ValidTarget, Variant as ValidVariant, source::Source as ResolvedSource,
};

impl ValidTarget {
    /// Construct validated semantics from one coherent parsed target.
    #[inline]
    pub fn from_input(target: InputTarget) -> syn::Result<Self> {
        match target {
            InputTarget::Struct(structure) => {
                let structure = structure.validate()?;

                Ok(Self::Struct(Box::new(structure)))
            }
            InputTarget::Enum(enumeration) => enumeration.validate().map(Self::Enum),
        }
    }
}

impl InputStructure {
    /// Resolve this parsed structure into validated semantics.
    fn validate(self) -> syn::Result<ValidStructure> {
        let (config, name, generics, fields, declaration) = self.parts();

        let (inline, import) = config.parts();

        let (display, source, conversion) = Declaration::new(&fields, declaration).validate()?;

        let header = Header::new(inline, import, name, generics);
        let structure = ValidStructure::new(header, fields, display, source, conversion);

        Ok(structure)
    }
}

impl InputEnumeration {
    /// Resolve every parsed variant before constructing enumeration semantics.
    fn validate(self) -> syn::Result<ValidEnumeration> {
        let (config, name, generics, variants) = self.parts();

        let (inline, import) = config.parts();

        let variants = variants.into_iter().map(InputVariant::validate).collect::<syn::Result<Vec<_>>>()?;
        let header = Header::new(inline, import, name, generics);
        let enumeration = ValidEnumeration::new(header, variants);

        Ok(enumeration)
    }
}

impl InputVariant {
    /// Resolve this parsed variant into validated semantics.
    fn validate(self) -> syn::Result<ValidVariant> {
        let (name, fields, declaration) = self.parts();

        let (display, source, conversion) = Declaration::new(&fields, declaration).validate()?;

        let variant = ValidVariant::new(name, fields, display, source, conversion);

        Ok(variant)
    }
}

/// Field dependent validation for one coherent parsed declaration.
// NOTE(invariant): The declaration is validated only against the borrowed field collection.
struct Declaration<'fields> {
    /// Fields against which selectors and captures are resolved.
    fields: &'fields Fields,

    /// Syntactically coherent source declaration.
    input: InputDeclaration,
}

impl<'fields> Declaration<'fields> {
    /// Construct one field dependent validation request.
    #[inline]
    const fn new(fields: &'fields Fields, input: InputDeclaration) -> Self {
        Self { fields, input }
    }

    /// Resolve all field dependent behavior into semantic state.
    fn validate(self) -> syn::Result<(Display, ErrorSource, Option<Conversion>)> {
        let Self { fields, input } = self;

        match input {
            InputDeclaration::Transparent(field) => Self::transparent(fields, field),
            InputDeclaration::Ordinary(ordinary) => {
                let (display, source) = ordinary.parts();

                let display = Self::display(fields, display)?;

                let (source, conversion) = Self::source(fields, source)?;

                Ok((display, source, conversion))
            }
        }
    }

    /// Resolve one ordinary display declaration.
    fn display(fields: &Fields, display: InputDisplay) -> syn::Result<Display> {
        match display {
            InputDisplay::Format(format) => Resolved::resolve(format, fields).map(Display::Format),
            InputDisplay::Custom(path) => Ok(Display::Custom(path)),
        }
    }

    /// Resolve ordinary source and conversion behavior.
    fn source(fields: &Fields, source: InputSource) -> syn::Result<(ErrorSource, Option<Conversion>)> {
        match source {
            InputSource::None => Ok((ErrorSource::None, None)),
            InputSource::Explicit(field) => {
                let field = fields.resolve(field)?;
                let source = ResolvedSource::new(field);

                Ok((ErrorSource::Field(source), None))
            }
            InputSource::From(from) => {
                let conversion = Self::conversion(fields, from.span())?;
                let source = ResolvedSource::new(conversion.field().clone());

                Ok((ErrorSource::Field(source), Some(conversion)))
            }
            InputSource::ExplicitFrom { field, from } => {
                let span = field.span();
                let conversion = Self::conversion(fields, from.span())?;
                let explicit = fields.resolve(field)?;

                match &explicit == conversion.field() {
                    true => {
                        let source = ResolvedSource::new(explicit);

                        Ok((ErrorSource::Field(source), Some(conversion)))
                    }
                    false => Err(syn::Error::new(
                        span,
                        "`from` conversion and explicit source must refer to the same field",
                    )),
                }
            }
        }
    }

    /// Resolve transparent display and source behavior.
    fn transparent(fields: &Fields, field: FieldRef) -> syn::Result<(Display, ErrorSource, Option<Conversion>)> {
        let span = field.span();
        let field = fields.resolve(field)?;
        let source = ResolvedSource::new(field.clone());

        source.into_transparent().map_or_else(
            || Err(syn::Error::new(span, "transparent error source cannot be optional")),
            |source| {
                let display = Display::Transparent(field);
                let source = ErrorSource::Transparent(source);

                Ok((display, source, None))
            },
        )
    }

    /// Construct conversion semantics from the sole declared field.
    fn conversion(fields: &Fields, fallback: proc_macro2::Span) -> syn::Result<Conversion> {
        let field = fields.sole(fallback)?;

        Ok(Conversion::new(field))
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use proc_macro2::Span;
    use syn::DeriveInput;

    use crate::input::Target as InputTarget;

    use super::ValidTarget;

    /// Unknown source fields survive input and fail semantic validation.
    #[test]
    fn unknown_source_fails_during_validation() -> syn::Result<()> {
        let input: DeriveInput = syn::parse_str(
            r#"
            #[error("outer")]
            #[error(source(missing))]
            struct Error { inner: u8 }
            "#,
        )?;

        let target = InputTarget::parse(&input)?;

        let Err(error) = ValidTarget::from_input(target) else {
            return Err(syn::Error::new(
                input.ident.span(),
                "unknown source must fail during semantic validation",
            ));
        };

        match error.to_string().contains("unknown named field") {
            true => Ok(()),
            false => Err(syn::Error::new(Span::call_site(), "semantic failure must report the unknown field")),
        }
    }
}
