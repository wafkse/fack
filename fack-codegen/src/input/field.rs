//! Structural fields and resolved field proofs.
//!
//! `Fields` owns the source declaration shape. Resolution produces an owned
//! `Field` value that carries every fact later stages need, so semantic code
//! never reinterprets a bare index against an unrelated collection.

use alloc::vec::Vec;

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    FieldsNamed, FieldsUnnamed, Ident, LitInt, Type,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

/// One field resolved from a structural field collection.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// NOTE(invariant): Construction is owned by `Fields`, so position, source name,
// and type come from one declaration slot.
pub struct Field {
    /// Zero based declaration position.
    index: usize,

    /// Source identifier for a named field.
    name: Option<Ident>,

    /// Declared Rust type of the field.
    ty: Type,
}

impl Field {
    /// Construct one resolved field proof.
    #[inline]
    const fn new(index: usize, name: Option<Ident>, ty: Type) -> Self {
        Self { index, name, ty }
    }

    /// Return the zero based declaration position.
    #[inline]
    #[must_use]
    pub const fn index(&self) -> usize {
        let &Self { index, .. } = self;

        index
    }

    /// Return the source identifier when the field is named.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> Option<&Ident> {
        let &Self { ref name, .. } = self;

        name.as_ref()
    }

    /// Return the declared Rust type.
    #[inline]
    #[must_use]
    pub const fn ty(&self) -> &Type {
        let &Self { ref ty, .. } = self;

        ty
    }
}

/// A source level reference to a field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldRef {
    /// A named field reference.
    Named(Ident),

    /// A tuple field reference.
    Indexed {
        /// Parsed zero based field index.
        index: usize,

        /// Source literal that supplied the index.
        literal: LitInt,
    },
}

impl Parse for FieldRef {
    /// Parse either a named selector or a zero based tuple index.
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        let kind = (lookahead.peek(Ident), lookahead.peek(LitInt));

        match kind {
            (true, _) => input.parse().map(Self::Named),
            (false, true) => {
                let literal: LitInt = input.parse()?;
                let index = literal
                    .base10_parse::<usize>()
                    .map_err(|_error| syn::Error::new_spanned(&literal, "expected valid field index"))?;

                Ok(Self::Indexed { index, literal })
            }
            (false, false) => Err(lookahead.error()),
        }
    }
}

impl ToTokens for FieldRef {
    /// Reproduce the source selector for diagnostic token context.
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            &Self::Named(ref name) => name.to_tokens(tokens),
            &Self::Indexed { ref literal, .. } => literal.to_tokens(tokens),
        }
    }
}

/// One named field in source declaration order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// NOTE(invariant): The identifier and type come from the same named source
// field and preserve declaration order.
pub struct NamedField {
    /// Source identifier of the field.
    name: Ident,

    /// Declared Rust type of the field.
    ty: Type,
}

impl NamedField {
    /// Return the source identifier.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &Ident {
        let &Self { ref name, .. } = self;

        name
    }

    /// Return the declared Rust type.
    #[inline]
    #[must_use]
    pub const fn ty(&self) -> &Type {
        let &Self { ref ty, .. } = self;

        ty
    }
}

/// Structural fields for one source structure or enum variant.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Fields {
    /// Named fields in declaration order.
    Named(Vec<NamedField>),

    /// Tuple field types in declaration order.
    Unnamed(Vec<Type>),

    /// A unit structure or variant.
    Unit,
}

impl Fields {
    /// Convert `syn` fields into the structural field model.
    #[inline]
    pub fn from_syn(fields: &syn::Fields) -> syn::Result<Self> {
        match fields {
            &syn::Fields::Named(FieldsNamed { ref named, .. }) => {
                let fields = named
                    .iter()
                    .map(|field| {
                        let &syn::Field { ref ident, ref ty, .. } = field;

                        let name = ident
                            .clone()
                            .ok_or_else(|| syn::Error::new_spanned(ident, "expected a named field"))?;
                        let ty = ty.clone();

                        Ok(NamedField { name, ty })
                    })
                    .collect::<syn::Result<Vec<_>>>()?;

                Ok(Self::Named(fields))
            }
            &syn::Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
                let fields = unnamed.iter().map(|field| field.ty.clone()).collect();

                Ok(Self::Unnamed(fields))
            }
            &syn::Fields::Unit => Ok(Self::Unit),
        }
    }

    /// Return the number of declared fields.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        match self {
            &Self::Named(ref fields) => fields.len(),
            &Self::Unnamed(ref fields) => fields.len(),
            &Self::Unit => 0,
        }
    }

    /// Resolve every declared field in source order.
    #[inline]
    pub fn resolved(&self) -> impl Iterator<Item = Field> + '_ {
        (0..self.len()).filter_map(|index| self.at(index))
    }

    /// Resolve a source level selector against this collection.
    #[inline]
    pub fn resolve(&self, field: FieldRef) -> syn::Result<Field> {
        match field {
            FieldRef::Named(name) => self.named(&name),
            FieldRef::Indexed { index, literal } => self.indexed_at(index, literal.span()),
        }
    }

    /// Resolve the sole declared field.
    #[inline]
    pub fn sole(&self, fallback: Span) -> syn::Result<Field> {
        let span = self.declaration_span().unwrap_or(fallback);

        match self.len() {
            1 => self.at(0).ok_or_else(|| syn::Error::new(span, "expected exactly one field")),
            _ => Err(syn::Error::new(span, "expected exactly one field")),
        }
    }

    /// Resolve a named field against this collection.
    #[inline]
    pub fn named(&self, name: &Ident) -> syn::Result<Field> {
        match self {
            &Self::Named(ref fields) => fields
                .iter()
                .position(|field| field.name() == name)
                .and_then(|index| self.at(index))
                .ok_or_else(|| syn::Error::new_spanned(name, "unknown named field")),
            &Self::Unnamed(_) => Err(syn::Error::new_spanned(name, "named field reference used with tuple fields")),
            &Self::Unit => Err(syn::Error::new_spanned(name, "unit fields cannot contain a field reference")),
        }
    }

    /// Resolve a tuple field and attach a source-owned diagnostic span.
    fn indexed_at(&self, index: usize, span: Span) -> syn::Result<Field> {
        match self {
            &Self::Named(_) => Err(syn::Error::new(span, "indexed field reference used with named fields")),
            &Self::Unnamed(_) => self
                .at(index)
                .ok_or_else(|| syn::Error::new(span, "tuple field index is out of bounds")),
            &Self::Unit => Err(syn::Error::new(span, "unit fields cannot contain a field reference")),
        }
    }

    /// Resolve a tuple capture only when the collection is unnamed and in
    /// bounds.
    #[inline]
    #[must_use]
    pub fn tuple_capture(&self, index: usize) -> Option<Field> {
        match self {
            &Self::Unnamed(_) => self.at(index),
            &Self::Named(_) | &Self::Unit => None,
        }
    }

    /// Return the span covered by declared fields when one exists.
    fn declaration_span(&self) -> Option<Span> {
        match self {
            &Self::Named(ref fields) => {
                let first = fields.first()?.name().span();
                let last = fields.last()?.ty().span();

                Some(first.join(last).unwrap_or(first))
            }
            &Self::Unnamed(ref fields) => {
                let first = fields.first()?.span();
                let last = fields.last()?.span();

                Some(first.join(last).unwrap_or(first))
            }
            &Self::Unit => None,
        }
    }

    /// Resolve one declaration position into an owned field proof.
    fn at(&self, index: usize) -> Option<Field> {
        match self {
            &Self::Named(ref fields) => fields.get(index).map(|field| {
                let name = Some(field.name().clone());
                let ty = field.ty().clone();

                Field::new(index, name, ty)
            }),
            &Self::Unnamed(ref fields) => fields.get(index).map(|ty| {
                let name = None;
                let ty = ty.clone();

                Field::new(index, name, ty)
            }),
            &Self::Unit => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use syn::{Data, DeriveInput, parse_quote};

    use super::*;

    /// Return a diagnostic when a test requirement is not satisfied.
    fn verify(condition: bool, message: &'static str) -> syn::Result<()> {
        match condition {
            true => Ok(()),
            false => Err(syn::Error::new(Span::call_site(), message)),
        }
    }

    /// Named resolution carries position, source name, and type together.
    #[test]
    fn named_resolution_produces_complete_field_proof() -> syn::Result<()> {
        let input: DeriveInput = parse_quote!(
            struct Test {
                first: u8,
                second: u16,
            }
        );

        let Data::Struct(data) = input.data else {
            return Err(syn::Error::new(Span::call_site(), "test input must be a structure"));
        };

        let fields = Fields::from_syn(&data.fields)?;
        let expected_name: Ident = parse_quote!(second);
        let field = fields.named(&expected_name)?;
        let ty = field.ty();

        verify(field.index() == 1, "resolved named field must preserve declaration position")?;
        verify(
            field.name() == Some(&expected_name),
            "resolved named field must preserve its source name",
        )?;
        verify(
            quote::quote!(#ty).to_string() == "u16",
            "resolved named field must preserve its type",
        )?;

        Ok(())
    }

    /// Tuple resolution carries its source position and declared type.
    #[test]
    fn tuple_resolution_produces_complete_field_proof() -> syn::Result<()> {
        let input: DeriveInput = parse_quote!(
            struct Test(u8, u16);
        );

        let Data::Struct(data) = input.data else {
            return Err(syn::Error::new(Span::call_site(), "test input must be a structure"));
        };

        let fields = Fields::from_syn(&data.fields)?;
        let reference: FieldRef = parse_quote!(1);
        let field = fields.resolve(reference)?;
        let ty = field.ty();

        verify(field.index() == 1, "resolved tuple field must preserve declaration position")?;
        verify(field.name().is_none(), "resolved tuple field must remain unnamed")?;
        verify(
            quote::quote!(#ty).to_string() == "u16",
            "resolved tuple field must preserve its type",
        )?;

        Ok(())
    }
}
