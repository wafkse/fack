//! Parsed derive targets and attribute collection.

use alloc::vec::Vec;

use proc_macro2::Span;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Variant as SynVariant, spanned::Spanned};

use crate::{
    diagnostic::Errors,
    enumerate::{Enumeration, Variant},
    field::{FieldRef, Fields},
    format::Format,
    structure::Structure,
    syntax::{Config, Declaration, Import, Inline, Param, ParamKind, Transparent},
    validate::ValidatedTarget,
};

/// A parsed error declaration before semantic validation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Target(Kind);

/// Internal parsed target representation shared with validation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A parsed enumeration declaration.
    Enum(Enumeration),

    /// A parsed structure declaration.
    Struct(Structure),
}

impl Target {
    /// Parse an error target from a derive input.
    #[inline]
    pub fn input(input: &DeriveInput) -> syn::Result<Self> {
        match &input.data {
            Data::Struct(data) => Self::structure(input, data),
            Data::Enum(data) => Self::enumeration(input, data),
            Data::Union(..) => Err(syn::Error::new_spanned(&input.ident, "unions cannot derive Error")),
        }
    }

    /// Validate the parsed declaration and return proof-bearing semantics.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when declarations conflict or field references do
    /// not satisfy their semantic requirements.
    #[inline]
    pub fn validate(self) -> syn::Result<ValidatedTarget> {
        let Self(kind) = self;

        crate::validate::target(kind)
    }

    /// Parse a structure target and collect each declaration at most once.
    fn structure(input: &DeriveInput, data: &DataStruct) -> syn::Result<Self> {
        let (params, ..) = Param::classify(&input.attrs)?;
        let mut format = Bucket::<Format>::new(&input.ident);
        let mut display = Bucket::<syn::Path>::new(&input.ident);
        let mut source = Bucket::<FieldRef>::new(&input.ident);
        let mut transparent = Bucket::<Transparent>::new(&input.ident);
        let mut from = Bucket::<()>::new(&input.ident);
        let mut inline = Bucket::<Inline>::new(&input.ident);
        let mut import = Bucket::<Import>::new(&input.ident);

        for param in params {
            let (name, kind) = param.parts();
            match kind {
                ParamKind::Format(value) => format.push((name, value)),
                ParamKind::Display(value) => display.push((name, value)),
                ParamKind::Source(value) => source.push((name, value)),
                ParamKind::Transparent(value) => transparent.push((name, value)),
                ParamKind::From => from.push((name, ())),
                ParamKind::Inline(value) => inline.push((name, value)),
                ParamKind::Import(value) => import.push((name, value)),
            }
        }

        let inline = inline.optional()?;
        let import = import.optional()?;
        let name = input.ident.clone();
        let generics = input.generics.clone();
        let fields = Fields::from_syn(&data.fields)?;
        let format = format.optional()?;
        let display = display.optional()?;
        let source = source.optional()?;
        let transparent = transparent.optional()?;
        let from = from.optional()?.is_some();
        let config = Config::new(inline, import);
        let declaration = Declaration::new(format, display, source, transparent, from);
        let structure = Structure::new(config, name, generics, fields, declaration);

        Ok(Self(Kind::Struct(structure)))
    }

    /// Parse enum-level options and validate that behavior parameters stay on
    /// variants.
    fn enumeration(input: &DeriveInput, data: &DataEnum) -> syn::Result<Self> {
        let (params, ..) = Param::classify(&input.attrs)?;
        let mut inline = Bucket::<Inline>::new(&input.ident);
        let mut import = Bucket::<Import>::new(&input.ident);
        let mut errors = Errors::new();

        for param in params {
            let (name, kind) = param.parts();
            match kind {
                ParamKind::Inline(value) => inline.push((name, value)),
                ParamKind::Import(value) => import.push((name, value)),
                _ => errors.push(syn::Error::new_spanned(
                    name,
                    "only `inline` and `import` are valid on an error enum",
                )),
            }
        }

        let mut variants = Vec::with_capacity(data.variants.len());

        for variant in &data.variants {
            match Self::variant(variant) {
                Ok(variant) => variants.push(variant),
                Err(error) => errors.push(error),
            }
        }

        let inline = inline.optional()?;
        let import = import.optional()?;
        let name = input.ident.clone();
        let generics = input.generics.clone();
        let config = Config::new(inline, import);
        let enumeration = Enumeration::new(config, name, generics, variants);

        errors.finish(Self(Kind::Enum(enumeration)))
    }

    /// Parse one enum variant and reject container-only parameters.
    fn variant(variant: &SynVariant) -> syn::Result<Variant> {
        let (params, ..) = Param::classify(&variant.attrs)?;
        let mut format = Bucket::<Format>::new(&variant.ident);
        let mut display = Bucket::<syn::Path>::new(&variant.ident);
        let mut source = Bucket::<FieldRef>::new(&variant.ident);
        let mut transparent = Bucket::<Transparent>::new(&variant.ident);
        let mut from = Bucket::<()>::new(&variant.ident);
        let mut errors = Errors::new();

        for param in params {
            let (name, kind) = param.parts();
            match kind {
                ParamKind::Format(value) => format.push((name, value)),
                ParamKind::Display(value) => display.push((name, value)),
                ParamKind::Source(value) => source.push((name, value)),
                ParamKind::Transparent(value) => transparent.push((name, value)),
                ParamKind::From => from.push((name, ())),
                ParamKind::Inline(..) | ParamKind::Import(..) => {
                    errors.push(syn::Error::new_spanned(name, "this parameter belongs on the enum"))
                }
            }
        }

        let name = variant.ident.clone();
        let fields = Fields::from_syn(&variant.fields)?;
        let format = format.optional()?;
        let display = display.optional()?;
        let source = source.optional()?;
        let transparent = transparent.optional()?;
        let from = from.optional()?.is_some();
        let declaration = Declaration::new(format, display, source, transparent, from);
        let variant = Variant::new(name, fields, declaration);

        errors.finish(variant)
    }
}

/// One syntax parameter constrained to at most one occurrence.
// NOTE(invariant): `first` stores the first occurrence and `extra` stores only
// later occurrences. `optional` is the sole transition that exposes a value.
struct Bucket<ValueType> {
    /// First occurrence of the syntax parameter.
    first: Option<(Span, ValueType)>,

    /// Later occurrences retained for duplicate diagnostics.
    extra: Vec<(Span, ValueType)>,
}

impl<ValueType> Bucket<ValueType> {
    /// Construct an empty duplicate-detection bucket.
    const fn new<SubjectType>(_subject: &SubjectType) -> Self
    where
        SubjectType: Spanned,
    {
        let first = None;
        let extra = Vec::new();

        Self { first, extra }
    }

    /// Record one occurrence while preserving the first span for diagnostics.
    fn push<SpanType>(&mut self, (spanned, value): (SpanType, ValueType))
    where
        SpanType: Spanned,
    {
        let Self { first, extra } = self;
        let span = spanned.span();

        if first.is_some() {
            extra.push((span, value));
        } else {
            *first = Some((span, value));
        }
    }

    /// Return the unique value or combine diagnostics for duplicate
    /// occurrences.
    fn optional(self) -> syn::Result<Option<ValueType>> {
        let Self { first, extra } = self;

        match first {
            Some((_span, value)) if extra.is_empty() => Ok(Some(value)),
            Some((span, _)) => {
                let mut error = syn::Error::new(span, "parameter declared more than once");

                for (extra_span, _) in extra {
                    error.combine(syn::Error::new(extra_span, "duplicate parameter"));
                }

                Err(error)
            }
            None => Ok(None),
        }
    }
}
