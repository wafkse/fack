//! Pattern binding for generated structure and variant matches.

use alloc::{format, vec::Vec};

use proc_macro2::TokenStream;
use syn::Ident;

use crate::field::{FieldId, Fields, Shape};

/// One generated local binding for a resolved field.
#[derive(Clone, Debug)]
pub struct Binding {
    /// Resolved field represented by this local.
    field: FieldId,

    /// Generated local identifier.
    ident: Ident,
}

impl Binding {
    /// Return the resolved field represented by this binding.
    #[inline]
    #[must_use]
    pub const fn field(&self) -> FieldId {
        let Self { field, .. } = self;

        *field
    }

    /// Return the generated local identifier.
    #[inline]
    #[must_use]
    pub const fn ident(&self) -> &Ident {
        let Self { ident, .. } = self;

        ident
    }
}

/// A generated match pattern together with every local it establishes.
// NOTE(invariant): Every stored binding occurs in `pattern` exactly once and
// refers to a field requested by the originating `Bind::bind` call.
#[derive(Clone, Debug)]
pub struct Bindings {
    /// Generated destructuring pattern.
    pattern: TokenStream,

    /// Selected fields and the locals established by the pattern.
    bindings: Vec<Binding>,
}

impl Bindings {
    /// Return the generated destructuring pattern.
    #[inline]
    #[must_use]
    pub const fn pattern(&self) -> &TokenStream {
        let Self { pattern, .. } = self;

        pattern
    }

    /// Return the binding for one selected field.
    #[must_use]
    #[inline]
    pub fn get(&self, field: FieldId) -> Option<&Binding> {
        let Self { bindings, .. } = self;

        bindings.iter().find(|binding| binding.field() == field)
    }

    /// Return the generated identifier for one selected field.
    #[must_use]
    #[inline]
    pub fn ident(&self, field: FieldId) -> &Ident {
        self.get(field)
            .map(Binding::ident)
            .expect("selected field must have a generated binding")
    }
}

/// Generate local bindings for a selected set of fields.
pub trait Bind {
    /// Generate one destructuring pattern and its established locals.
    fn bind(&self, selected: &[FieldId]) -> Bindings;
}

impl Bind for Fields {
    /// Build one destructuring pattern containing exactly the requested fields.
    #[inline]
    fn bind(&self, selected: &[FieldId]) -> Bindings {
        let bindings = selected
            .iter()
            .copied()
            .map(|field| {
                let ident = match self.name(field) {
                    Some(name) => name.clone(),
                    None => Ident::new(&format!("_{index}", index = field.index()), proc_macro2::Span::call_site()),
                };

                Binding { field, ident }
            })
            .collect::<Vec<_>>();

        let pattern = match self.shape() {
            Shape::Named => {
                let fields = bindings.iter().map(|binding| {
                    let ident = binding.ident();

                    quote::quote! { ref #ident }
                });
                let fields = fields.collect::<Vec<_>>();

                if fields.is_empty() {
                    quote::quote! { { .. } }
                } else {
                    quote::quote! { { #(#fields),*, .. } }
                }
            }
            Shape::Unnamed => {
                let fields = (0..self.len()).map(|index| {
                    let field = FieldId::from_index(index);

                    match bindings.iter().find(|binding| binding.field() == field) {
                        Some(binding) => {
                            let ident = binding.ident();

                            quote::quote! { ref #ident }
                        }
                        None => quote::quote! { _ },
                    }
                });

                quote::quote! { ( #(#fields),* ) }
            }
            Shape::Unit => quote::quote! {},
        };

        Bindings { pattern, bindings }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::{Bind as _, *};

    /// Named patterns omit fields that generation does not consume.
    #[test]
    fn named_bindings_select_only_requested_fields() {
        let input: syn::DeriveInput = syn::parse_quote!(
            struct Test {
                first: u8,
                second: u16,
            }
        );
        let syn::Data::Struct(data) = input.data else {
            unreachable!("test input is a structure");
        };
        let fields = Fields::from_syn(&data.fields).expect("test fields must parse");
        let bindings = fields.bind(&[FieldId::from_index(1)]);
        let pattern = bindings.pattern().to_string();

        assert!(pattern.contains("second"));
        assert!(!pattern.contains("first"));
    }

    /// Tuple patterns preserve positional holes around selected fields.
    #[test]
    fn tuple_bindings_preserve_field_positions() {
        let input: syn::DeriveInput = syn::parse_quote!(
            struct Test(u8, u16, u32);
        );
        let syn::Data::Struct(data) = input.data else {
            unreachable!("test input is a structure");
        };
        let fields = Fields::from_syn(&data.fields).expect("test fields must parse");
        let bindings = fields.bind(&[FieldId::from_index(1)]);
        let pattern = bindings.pattern().to_string();

        assert!(pattern.contains("_1"));
        assert!(pattern.starts_with('('));
    }
}
