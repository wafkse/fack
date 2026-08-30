//! Parsed derive targets and attribute collection.
//!
//! Target parsing gathers helper attributes by location, rejects duplicates,
//! and constructs structure or enum input states only after syntax diagnostics
//! are clear. Field-dependent facts remain intentionally unresolved.

use alloc::vec::Vec;
use core::mem::replace;

use proc_macro2::Span;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Ident, Variant as SynVariant};

use super::format::Format;

use super::{
    attribute::{Config, Declaration, Import, InlinePolicy, Param, ParamKind, Transparent},
    diagnostic::Errors,
    field::{FieldRef, Fields},
};

/// A parsed error declaration before semantic validation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// A parsed enumeration declaration.
    Enum(Enumeration),

    /// A parsed structure declaration.
    Struct(Structure),
}

impl Target {
    /// Parse an error target from a derive input.
    #[inline]
    pub fn parse(input: &DeriveInput) -> syn::Result<Self> {
        match &input.data {
            &Data::Struct(ref data) => Self::structure(input, data),
            &Data::Enum(ref data) => Self::enumeration(input, data),
            &Data::Union(..) => Err(syn::Error::new_spanned(&input.ident, "unions cannot derive Error")),
        }
    }

    /// Parse a structure target and collect each declaration at most once.
    fn structure(input: &DeriveInput, data: &DataStruct) -> syn::Result<Self> {
        let params = Param::classify(&input.attrs)?;
        let mut format = Occurrence::<Format>::new();
        let mut display = Occurrence::<syn::Path>::new();
        let mut source = Occurrence::<FieldRef>::new();
        let mut transparent = Occurrence::<Transparent>::new();
        let mut from = Occurrence::<Ident>::new();
        let mut inline = Occurrence::<InlinePolicy>::new();
        let mut import = Occurrence::<Import>::new();

        for param in params {
            let span = param.span();

            let (_name, kind) = param.parts();

            match kind {
                ParamKind::Format(value) => format.push((span, value)),
                ParamKind::Display(value) => display.push((span, value)),
                ParamKind::Source(value) => source.push((span, value)),
                ParamKind::Transparent(value) => transparent.push((span, value)),
                ParamKind::From(value) => from.push((span, value)),
                ParamKind::Inline(value) => inline.push((span, value)),
                ParamKind::Import(value) => import.push((span, value)),
            }
        }

        let mut errors = Errors::new();
        let format = format.resolve_spanned(&mut errors);
        let display = display.resolve_spanned(&mut errors);
        let source = source.resolve_spanned(&mut errors);
        let transparent = transparent.resolve_spanned(&mut errors);
        let from = from.resolve_spanned(&mut errors);
        let inline = inline.resolve(&mut errors);
        let import = import.resolve(&mut errors);

        errors.and_then(|| {
            let name = input.ident.clone();
            let generics = input.generics.clone();
            let fields = Fields::from_syn(&data.fields)?;
            let config = Config::new(inline, import);
            let declaration = Declaration::classify(&input.ident, format, display, source, transparent, from)?;
            let structure = Structure::new(config, name, generics, fields, declaration);

            Ok(Self::Struct(structure))
        })
    }

    /// Parse enum level options and reject behavior parameters on the enum.
    fn enumeration(input: &DeriveInput, data: &DataEnum) -> syn::Result<Self> {
        let params = Param::classify(&input.attrs)?;
        let mut inline = Occurrence::<InlinePolicy>::new();
        let mut import = Occurrence::<Import>::new();
        let mut errors = Errors::new();

        for param in params {
            let span = param.span();

            let (_name, kind) = param.parts();

            match kind {
                ParamKind::Inline(value) => inline.push((span, value)),
                ParamKind::Import(value) => import.push((span, value)),
                _ => errors.push(syn::Error::new(span, "only `inline` and `import` are valid on an error enum")),
            }
        }

        let mut variants = Vec::with_capacity(data.variants.len());

        for variant in &data.variants {
            match Self::variant(variant) {
                Ok(variant) => variants.push(variant),
                Err(error) => errors.push(error),
            }
        }

        let inline = inline.resolve(&mut errors);
        let import = import.resolve(&mut errors);

        errors.and_then(|| {
            let name = input.ident.clone();
            let generics = input.generics.clone();
            let config = Config::new(inline, import);
            let enumeration = Enumeration {
                config,
                name,
                generics,
                variants,
            };

            Ok(Self::Enum(enumeration))
        })
    }

    /// Parse one enum variant and reject container only parameters.
    fn variant(variant: &SynVariant) -> syn::Result<Variant> {
        let params = Param::classify(&variant.attrs)?;
        let mut format = Occurrence::<Format>::new();
        let mut display = Occurrence::<syn::Path>::new();
        let mut source = Occurrence::<FieldRef>::new();
        let mut transparent = Occurrence::<Transparent>::new();
        let mut from = Occurrence::<Ident>::new();
        let mut errors = Errors::new();

        for param in params {
            let span = param.span();

            let (_name, kind) = param.parts();

            match kind {
                ParamKind::Format(value) => format.push((span, value)),
                ParamKind::Display(value) => display.push((span, value)),
                ParamKind::Source(value) => source.push((span, value)),
                ParamKind::Transparent(value) => transparent.push((span, value)),
                ParamKind::From(value) => from.push((span, value)),
                ParamKind::Inline(..) | ParamKind::Import(..) => {
                    errors.push(syn::Error::new(span, "this parameter belongs on the enum"));
                }
            }
        }

        let format = format.resolve_spanned(&mut errors);
        let display = display.resolve_spanned(&mut errors);
        let source = source.resolve_spanned(&mut errors);
        let transparent = transparent.resolve_spanned(&mut errors);
        let from = from.resolve_spanned(&mut errors);

        errors.and_then(|| {
            let name = variant.ident.clone();
            let fields = Fields::from_syn(&variant.fields)?;
            let declaration = Declaration::classify(&variant.ident, format, display, source, transparent, from)?;
            let variant = Variant { name, fields, declaration };

            Ok(variant)
        })
    }
}

/// Occurrence state for one syntax parameter.
enum Occurrence<ValueType> {
    /// The parameter was not declared.
    Missing,

    /// The parameter was declared exactly once.
    One {
        /// Span of the unique declaration.
        span: Span,

        /// Parsed value of the unique declaration.
        value: ValueType,
    },

    /// The parameter was declared more than once.
    Duplicate {
        /// Span of the first declaration.
        first: Span,

        /// Spans of every later declaration.
        extra: Vec<Span>,
    },
}

impl<ValueType> Occurrence<ValueType> {
    /// Construct an empty occurrence state.
    const fn new() -> Self {
        Self::Missing
    }

    /// Record one occurrence and retain only spans after duplication occurs.
    fn push(&mut self, (span, value): (Span, ValueType)) {
        let current = replace(self, Self::Missing);
        let next = match current {
            Self::Missing => Self::One { span, value },
            Self::One { span: first, .. } => Self::Duplicate {
                first,
                extra: alloc::vec![span],
            },
            Self::Duplicate { first, mut extra } => {
                extra.push(span);

                Self::Duplicate { first, extra }
            }
        };

        *self = next;
    }

    /// Resolve the unique value and accumulate any duplicate diagnostic.
    fn resolve(self, errors: &mut Errors) -> Option<ValueType> {
        self.resolve_spanned(errors).map(|(_, value)| value)
    }

    /// Resolve the unique value together with its source occurrence span.
    fn resolve_spanned(self, errors: &mut Errors) -> Option<(Span, ValueType)> {
        match self {
            Self::Missing => None,
            Self::One { span, value } => Some((span, value)),
            Self::Duplicate { first, extra } => {
                let mut error = syn::Error::new(first, "parameter declared more than once");

                for span in extra {
                    error.combine(syn::Error::new(span, "duplicate parameter"));
                }

                errors.push(error);
                None
            }
        }
    }
}

/// A parsed structure error declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// NOTE(invariant): Attributes and fields belong to the same parsed structure
// and have passed syntax classification.
pub struct Structure {
    /// Parsed container wide generation options.
    config: Config,

    /// Source structure identifier.
    name: syn::Ident,

    /// Source generic parameters.
    generics: syn::Generics,

    /// Parsed structural fields.
    fields: Fields,

    /// Parsed error behavior declarations.
    declaration: Declaration,
}

impl Structure {
    /// Construct a parsed structure declaration.
    #[inline]
    #[must_use]
    const fn new(config: Config, name: syn::Ident, generics: syn::Generics, fields: Fields, declaration: Declaration) -> Self {
        Self {
            config,
            name,
            generics,
            fields,
            declaration,
        }
    }

    /// Consume the structure into its parsed components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Config, syn::Ident, syn::Generics, Fields, Declaration) {
        let Self {
            config,
            name,
            generics,
            fields,
            declaration,
        } = self;

        (config, name, generics, fields, declaration)
    }
}

/// A parsed enumeration error declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// NOTE(invariant): Global configuration and variants belong to the same parsed
// enumeration.
pub struct Enumeration {
    /// Parsed container wide generation options.
    config: Config,

    /// Source enumeration identifier.
    name: syn::Ident,

    /// Source generic parameters.
    generics: syn::Generics,

    /// Parsed error variants.
    variants: Vec<Variant>,
}

impl Enumeration {
    /// Consume the enumeration into its parsed components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (Config, syn::Ident, syn::Generics, Vec<Variant>) {
        let Self {
            config,
            name,
            generics,
            variants,
        } = self;

        (config, name, generics, variants)
    }
}

/// A parsed error enumeration variant.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// NOTE(invariant): The fields and declaration belong to the same parsed variant
// after syntax classification.
pub struct Variant {
    /// Source variant identifier.
    name: syn::Ident,

    /// Parsed variant fields.
    fields: Fields,

    /// Parsed error behavior declarations.
    declaration: Declaration,
}

impl Variant {
    /// Consume the variant into its parsed components.
    #[inline]
    #[must_use]
    pub fn parts(self) -> (syn::Ident, Fields, Declaration) {
        let Self { name, fields, declaration } = self;

        (name, fields, declaration)
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use proc_macro2::Span;
    use syn::DeriveInput;

    use super::Target;

    /// Syntax-only display conflicts fail during input classification.
    #[test]
    fn conflicting_display_syntax_fails_during_input() -> syn::Result<()> {
        let input: DeriveInput = syn::parse_str(
            r#"
            #[error("outer")]
            #[error(display(render))]
            struct Error;
            "#,
        )?;

        let Err(error) = Target::parse(&input) else {
            return Err(syn::Error::new(
                input.ident.span(),
                "conflicting display syntax must fail during input classification",
            ));
        };

        match error.to_string().contains("both a format string and `display(...)`") {
            true => Ok(()),
            false => Err(syn::Error::new(
                Span::call_site(),
                "input conflict must report the expected diagnostic",
            )),
        }
    }
}
