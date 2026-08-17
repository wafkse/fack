//! Field identity, lookup, and representation.

use alloc::{string::ToString, vec::Vec};

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    FieldsNamed, FieldsUnnamed, Ident, LitInt, Type,
    parse::{Parse, ParseStream},
};

use crate::resolve::Resolve;

/// A stable field identity established after lookup succeeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldId(usize);

impl FieldId {
    /// Return the zero-based field index.
    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        let Self(index) = self;

        index
    }

    /// Construct a field identity from a proven zero-based index.
    #[inline]
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }
}

/// A source-level reference to a field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldRef {
    /// A named field reference.
    Named(Ident),

    /// A tuple field reference.
    Indexed(usize),
}

impl Parse for FieldRef {
    /// Parse either a named field selector or a zero-based tuple index.
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Ident) {
            return input.parse().map(Self::Named);
        }

        if lookahead.peek(LitInt) {
            let literal: LitInt = input.parse()?;
            let index = literal
                .base10_parse::<usize>()
                .map_err(|_| syn::Error::new_spanned(&literal, "expected valid field index"))?;

            return Ok(Self::Indexed(index));
        }

        Err(lookahead.error())
    }
}

impl ToTokens for FieldRef {
    /// Reproduce the source-level selector when diagnostics need token context.
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Named(name) => name.to_tokens(tokens),
            Self::Indexed(index) => {
                LitInt::new(index.to_string().as_str(), Span::call_site()).to_tokens(tokens);
            }
        }
    }
}

/// The structural shape of a field collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    /// Named fields.
    Named,

    /// Tuple fields.
    Unnamed,

    /// No fields.
    Unit,
}

/// One field in a structural collection.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Field {
    /// Source field identifier when this is a named field.
    name: Option<Ident>,

    /// Source field type.
    ty: Type,
}

impl Field {
    /// Return the source-level field name when one exists.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> Option<&Ident> {
        let Self { name, .. } = self;

        name.as_ref()
    }

    /// Return the field type.
    #[inline]
    #[must_use]
    pub const fn ty(&self) -> &Type {
        let Self { ty, .. } = self;

        ty
    }
}

/// A structural field collection.
// NOTE(invariant): `shape` agrees with every stored field name. Named collections
// contain only named fields, unnamed collections contain only unnamed fields,
// and unit collections contain no fields. Construction establishes this once.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fields {
    /// Structural shape shared by every stored field.
    shape: Shape,

    /// Fields in source declaration order.
    fields: Vec<Field>,
}

impl Fields {
    /// Convert `syn` fields into the fack field model.
    #[inline]
    pub fn from_syn(fields: &syn::Fields) -> syn::Result<Self> {
        match fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => {
                let fields = named
                    .iter()
                    .map(|syn::Field { ident, ty, .. }| {
                        let name = ident
                            .clone()
                            .ok_or_else(|| syn::Error::new_spanned(ident, "expected a named field"))?;
                        let ty = ty.clone();

                        Ok(Field { name: Some(name), ty })
                    })
                    .collect::<syn::Result<Vec<_>>>()?;
                let shape = Shape::Named;

                Ok(Self { shape, fields })
            }
            syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                let fields = unnamed
                    .iter()
                    .map(|syn::Field { ty, .. }| Field {
                        name: None,
                        ty: ty.clone(),
                    })
                    .collect();
                let shape = Shape::Unnamed;

                Ok(Self { shape, fields })
            }
            syn::Fields::Unit => {
                let shape = Shape::Unit;
                let fields = Vec::new();

                Ok(Self { shape, fields })
            }
        }
    }

    /// Return the structural field shape.
    #[inline]
    #[must_use]
    pub const fn shape(&self) -> Shape {
        let Self { shape, .. } = self;

        *shape
    }

    /// Return the number of fields.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        let Self { fields, .. } = self;

        fields.len()
    }

    /// Return one resolved field.
    #[inline]
    #[must_use]
    pub fn get(&self, field: FieldId) -> &Field {
        let Self { fields, .. } = self;

        fields
            .get(field.index())
            .expect("validated field identity must remain within its originating field collection")
    }

    /// Return the field type for one resolved field.
    #[inline]
    #[must_use]
    pub fn ty(&self, field: FieldId) -> &Type {
        self.get(field).ty()
    }

    /// Return the source-level name for one resolved field.
    #[inline]
    #[must_use]
    pub fn name(&self, field: FieldId) -> Option<&Ident> {
        self.get(field).name()
    }

    /// Resolve the sole field.
    #[inline]
    pub fn sole(&self) -> syn::Result<FieldId> {
        let Self { fields, .. } = self;

        if fields.len() == 1 {
            Ok(FieldId::from_index(0))
        } else {
            Err(syn::Error::new(Span::call_site(), "expected exactly one field"))
        }
    }

    /// Resolve a named field when this is a named collection.
    #[inline]
    pub fn named(&self, name: &Ident) -> syn::Result<FieldId> {
        let Self { shape, fields } = self;

        match shape {
            Shape::Named => fields
                .iter()
                .position(|field| field.name() == Some(name))
                .map(FieldId::from_index)
                .ok_or_else(|| syn::Error::new_spanned(name, "unknown named field")),
            Shape::Unnamed => Err(syn::Error::new_spanned(name, "named field reference used with tuple fields")),
            Shape::Unit => Err(syn::Error::new_spanned(name, "unit fields cannot contain a field reference")),
        }
    }

    /// Resolve a tuple field when this is an unnamed collection.
    #[inline]
    pub fn indexed(&self, index: usize) -> syn::Result<FieldId> {
        let Self { shape, fields } = self;

        match shape {
            Shape::Named => Err(syn::Error::new(Span::call_site(), "indexed field reference used with named fields")),
            Shape::Unnamed if index < fields.len() => Ok(FieldId::from_index(index)),
            Shape::Unnamed => Err(syn::Error::new(Span::call_site(), "tuple field index is out of bounds")),
            Shape::Unit => Err(syn::Error::new(Span::call_site(), "unit fields cannot contain a field reference")),
        }
    }
}

impl Resolve for FieldRef {
    /// Field collection against which the source selector is resolved.
    type Context = Fields;

    /// Stable field identity produced after lookup succeeds.
    type Output = FieldId;

    /// Resolve the source selector once so downstream code can use `FieldId`.
    #[inline]
    fn resolve(self, fields: &Self::Context) -> syn::Result<Self::Output> {
        match self {
            Self::Named(name) => fields.named(&name),
            Self::Indexed(index) => fields.indexed(index),
        }
    }
}
