//! Code generation for the `fack` crate.
//!
//! This is not a `proc-macro` crate, therefore, this can be freely depended
//! upon in both normal and procedural macro contexts.
#![no_std]
#![forbid(unsafe_code, missing_docs, rustdoc::all, clippy::all, clippy::pedantic)]

extern crate alloc;

use core::any;

use alloc::vec::Vec;

use proc_macro2::Span;

use syn::{Data, DataEnum, DataStruct, DeriveInput, Variant, spanned::Spanned};

use common::{FieldRef, Format, ImportRoot, InlineOptions, Param, ParamKind, Transparent};

use structure::{Structure, StructureOptions};

pub mod enumerate;

pub mod structure;

pub mod common;

pub mod expand;

/// A list of [`syn::Error`]s that accumulates them into a single one.
///
/// This can be reduced to a single [`syn::Error`] in the end, but can still be
/// devoid of errors and result in a success.
#[derive(Debug, Clone)]
struct ErrorList(Option<syn::Error>);

impl ErrorList {
    /// Construct a new, empty error list.
    #[inline]
    pub const fn empty() -> Self {
        Self(None)
    }

    /// Append the target [`syn::Error`] to this list.
    #[inline]
    pub fn append(&mut self, error: syn::Error) {
        let Self(target_container) = self;

        match target_container.as_mut() {
            Some(target_list) => target_list.combine(error),
            None => *target_container = Some(error),
        }
    }

    /// Compose this error list with some type `T`.
    ///
    /// Returns [`Err`] if this error list is not empty, [`Ok`] otherwise.
    #[inline]
    pub fn compose<T>(self: Self, value: T) -> syn::Result<T> {
        match self {
            Self(Some(target_error)) => Err(target_error),
            Self(None) => Ok(value),
        }
    }
}

/// An error-derivation target.
///
/// Is either an enumeration or a structure.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// An error type that is an enumeration.
    Enum(enumerate::Enumeration),

    /// An error type that is a structure.
    Struct(structure::Structure),
}

impl Target {
    /// Parse an error [`Target`] from a target [`DeriveInput`].
    pub fn input(input: &DeriveInput) -> syn::Result<Self> {
        struct ParamBucket<P> {
            /// The span of the subject parameter receptor.
            span_subject: Span,

            /// The target parameter that was found.
            first_found: Option<(Span, P)>,

            /// The extraneous parameters that were found.
            extra_found: Vec<(Span, P)>,
        }

        impl<P> ParamBucket<P> {
            /// Construct a new, empty parameter bucket.
            #[inline]
            pub fn empty() -> Self {
                Self {
                    span_subject: Span::call_site(),
                    first_found: None,
                    extra_found: Vec::new(),
                }
            }

            /// Construct a new, empty parameter bucket, but with an associated
            /// span.
            #[inline]
            pub fn spanned<S>(span_subject: S) -> Self
            where
                S: Spanned,
            {
                Self {
                    first_found: None,
                    extra_found: Vec::new(),
                    span_subject: span_subject.span(),
                }
            }

            /// Push the param `P` into this bucket.
            #[inline]
            pub fn push<T>(&mut self, (spanned, param): (T, P))
            where
                T: Spanned,
            {
                let &mut Self {
                    ref mut first_found,
                    ref mut extra_found,
                    ..
                } = self;

                let span = spanned.span();

                let value = (span, param);

                match first_found {
                    Some(_) => extra_found.push(value),
                    None => *first_found = Some(value),
                }
            }

            /// Expect either one or none of the parameters to be found.
            #[inline]
            pub fn one_or_nothing(self) -> syn::Result<Option<P>> {
                let Self {
                    first_found, extra_found, ..
                } = self;

                match first_found {
                    Some((span, param)) => {
                        if extra_found.is_empty() {
                            Ok(Some(param))
                        } else {
                            let mut error = syn::Error::new(span, "multiple parameters found");

                            for (span, _) in extra_found {
                                error.combine(syn::Error::new(span, "extraneous parameter"));
                            }

                            Err(error)
                        }
                    }
                    None => Ok(None),
                }
            }

            /// Expect exactly one of the parameters to be found.
            #[inline]
            pub fn only_one(self) -> syn::Result<P> {
                let Self {
                    span_subject,
                    first_found,
                    extra_found,
                } = self;

                match first_found {
                    Some((span, param)) => {
                        if extra_found.is_empty() {
                            Ok(param)
                        } else {
                            let mut error = syn::Error::new(span, "multiple parameters found");

                            for (span, _) in extra_found {
                                error.combine(syn::Error::new(span, "extraneous parameter"));
                            }

                            Err(error)
                        }
                    }
                    None => Err(syn::Error::new(
                        span_subject,
                        format_args!("{}: no parameters found, but one was expected", any::type_name::<P>()),
                    )),
                }
            }
        }

        let DeriveInput {
            attrs,
            ident: name_ident,
            generics,
            data,
            ..
        } = input;

        let mut error_list = ErrorList::empty();

        match data {
            Data::Struct(DataStruct { fields, .. }) => {
                let (param_list, ..) = common::Param::classify(attrs)?;

                let mut source_bucket = ParamBucket::<FieldRef>::empty();
                let mut format_bucket = ParamBucket::<Format>::empty();
                let mut inline_bucket = ParamBucket::<InlineOptions>::empty();
                let mut root_bucket = ParamBucket::<ImportRoot>::empty();

                let mut transparent_bucket = ParamBucket::<Transparent>::empty();
                let mut from_bucket = ParamBucket::<()>::empty();

                for Param { name, kind } in param_list {
                    match kind {
                        ParamKind::Source(source) => source_bucket.push((name, source)),
                        ParamKind::Format(format) => format_bucket.push((name, format)),
                        ParamKind::Inline(inline) => inline_bucket.push((name, inline)),
                        ParamKind::Import(root) => root_bucket.push((name, root)),
                        ParamKind::Transparent(transparent) => transparent_bucket.push((name, transparent)),
                        ParamKind::From => from_bucket.push((name, ())),
                    }
                }

                let source_field = source_bucket.one_or_nothing()?;
                let inline_opts = inline_bucket.one_or_nothing()?;
                let root_import = root_bucket.one_or_nothing()?;
                let transparent = transparent_bucket.one_or_nothing()?;

                let from = from_bucket.one_or_nothing()?;

                let options = match (transparent, from) {
                    (Some(transparent), None) => StructureOptions::Transparent(transparent),
                    (None, Some(..)) => {
                        let format_args = format_bucket.only_one()?;

                        let (field_ref, field_type) = structure::FieldList::exactly_one(fields)?;

                        StructureOptions::Forward {
                            source_field,
                            field_ref,
                            field_type,
                            format_args,
                        }
                    }
                    (None, None) => {
                        let format_args = format_bucket.only_one()?;

                        StructureOptions::Standalone { source_field, format_args }
                    }
                    (Some(_), Some(_)) => {
                        return Err(syn::Error::new_spanned(
                            name_ident,
                            "structure can't be both transparent and from-forwarded",
                        ));
                    }
                };

                let field_list = structure::FieldList::raw(fields)?;

                let generics = generics.clone();

                let name_ident = name_ident.clone();

                Ok(Target::Struct(Structure {
                    inline_opts,
                    root_import,
                    name_ident,
                    generics,
                    field_list,
                    options,
                }))
            }
            Data::Enum(DataEnum { variants, .. }) => {
                let (param_list, ..) = common::Param::classify(attrs)?;

                let mut inline_bucket = ParamBucket::<InlineOptions>::empty();
                let mut root_bucket = ParamBucket::<ImportRoot>::empty();

                for Param { name, kind } in param_list {
                    match kind {
                        ParamKind::Inline(inline) => inline_bucket.push((name, inline)),
                        ParamKind::Import(root) => root_bucket.push((name, root)),
                        _ => error_list.append(syn::Error::new_spanned(name, "extraneous parameter")),
                    }
                }

                let inline_opts = inline_bucket.one_or_nothing()?;
                let root_import = root_bucket.one_or_nothing()?;

                let mut variant_list = Vec::with_capacity(variants.len());

                for Variant { ident, attrs, fields, .. } in variants {
                    let (param_list, ..) = common::Param::classify(attrs)?;

                    let mut format_bucket = ParamBucket::<Format>::spanned(ident);
                    let mut source_bucket = ParamBucket::<FieldRef>::empty();
                    let mut transparent_bucket = ParamBucket::<Transparent>::empty();
                    let mut from_bucket = ParamBucket::<()>::empty();

                    for Param { name, kind } in param_list {
                        match kind {
                            ParamKind::Format(format) => format_bucket.push((name, format)),
                            ParamKind::Source(source_field) => source_bucket.push((name, source_field)),
                            ParamKind::Transparent(transparent) => transparent_bucket.push((name, transparent)),
                            ParamKind::From => from_bucket.push((name, ())),
                            _ => error_list.append(syn::Error::new_spanned(name, "extraneous parameter")),
                        }
                    }

                    let transparent = transparent_bucket.one_or_nothing()?;
                    let from = from_bucket.one_or_nothing()?;
                    let source_field = source_bucket.one_or_nothing()?;

                    let variant_name = ident.clone();

                    let variant = match (transparent, from) {
                        (Some(_), Some(_)) => {
                            error_list.append(syn::Error::new_spanned(
                                ident,
                                "variant can't be both transparent and from-forwarded",
                            ));

                            continue;
                        }
                        (None, Some(..)) => {
                            let format = format_bucket.only_one()?;

                            let (field_ref, field_type) = structure::FieldList::exactly_one(fields)?;

                            enumerate::Variant::Forward {
                                format,
                                variant_name,
                                field_ref,
                                field_type,
                            }
                        }
                        (Some(transparent), None) => enumerate::Variant::Transparent {
                            transparent,
                            variant_name,
                            field_list: structure::FieldList::raw(fields)?,
                        },
                        (None, None) => {
                            let format = format_bucket.only_one()?;

                            enumerate::Variant::Struct {
                                format,
                                source_field,
                                variant_name,
                                field_list: structure::FieldList::raw(fields)?,
                            }
                        }
                    };

                    variant_list.push(variant);
                }

                let generics = generics.clone();

                let name_ident = name_ident.clone();

                let enumeration = enumerate::Enumeration {
                    inline_opts,
                    root_import,
                    name_ident,
                    generics,
                    variant_list,
                };

                error_list.compose(enumeration).map(Target::Enum)
            }
            Data::Union(..) => Err(syn::Error::new_spanned(name_ident, "unions can't be errors")),
        }
    }
}
