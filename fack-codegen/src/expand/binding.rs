//! Pattern binding for generated structure and variant matches.
//!
//! Field binding and pattern rendering are expansion nodes under the parent
//! `Expand` contract. A single-field binding carries the established local
//! together with the pattern that introduces it.

use alloc::{format, vec::Vec};
use core::slice::from_ref;

use proc_macro2::Span;
use syn::Ident;

use crate::input::{Field, Fields};

use super::{Binding, Expand, FieldBinding, FieldPattern, FieldSelection};

impl Expand for FieldBinding<'_> {
    type Context = ();
    type Output = Binding;

    fn expand_with(self, (): Self::Context) -> syn::Result<Self::Output> {
        let FieldBinding(fields, field) = self;

        let selected = from_ref(field);
        let pattern = FieldPattern(fields, FieldSelection::Selected(selected)).expand()?;
        let ident = Fields::binding_ident(field);

        Ok(Binding { pattern, ident })
    }
}

impl Expand for FieldPattern<'_> {
    type Context = ();
    type Output = proc_macro2::TokenStream;

    fn expand_with(self, (): Self::Context) -> syn::Result<Self::Output> {
        let FieldPattern(fields, selection) = self;

        let all_fields;
        let selected = match selection {
            FieldSelection::Selected(selected) => selected,
            FieldSelection::All => {
                all_fields = fields.resolved().collect::<Vec<_>>();

                &all_fields
            }
        };

        let pattern = match fields {
            &Fields::Named(_) => {
                let fields = selected
                    .iter()
                    .map(|field| {
                        let ident = Fields::binding_ident(field);

                        quote::quote! { ref #ident }
                    })
                    .collect::<Vec<_>>();

                if fields.is_empty() {
                    quote::quote! { { .. } }
                } else {
                    quote::quote! { { #(#fields),*, .. } }
                }
            }
            &Fields::Unnamed(_) => {
                let fields = fields.resolved().map(|field| {
                    selected.iter().find(|selected| selected.index() == field.index()).map_or_else(
                        || quote::quote! { _ },
                        |selected| {
                            let ident = Fields::binding_ident(selected);

                            quote::quote! { ref #ident }
                        },
                    )
                });

                quote::quote! { ( #(#fields),* ) }
            }
            &Fields::Unit => quote::quote! {},
        };

        Ok(pattern)
    }
}

impl Fields {
    /// Derive the generated local name carried by one resolved field.
    fn binding_ident(field: &Field) -> Ident {
        match field.name() {
            Some(name) => name.clone(),
            None => Ident::new(&format!("_{index}", index = field.index()), Span::call_site()),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use core::slice::from_ref;

    use proc_macro2::Span;
    use syn::{Data, DeriveInput, parse_quote};

    use crate::{
        expand::{Expand as _, FieldPattern, FieldSelection},
        input::{FieldRef, Fields},
    };

    /// Return a diagnostic when a test requirement is not satisfied.
    fn verify(condition: bool, message: &'static str) -> syn::Result<()> {
        match condition {
            true => Ok(()),
            false => Err(syn::Error::new(Span::call_site(), message)),
        }
    }

    /// Named patterns omit fields that generation does not consume.
    #[test]
    fn named_bindings_select_only_requested_fields() -> syn::Result<()> {
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
        let second = fields.named(&parse_quote!(second))?;
        let pattern = FieldPattern(&fields, FieldSelection::Selected(from_ref(&second)))
            .expand()?
            .to_string();

        let indexed: FieldRef = parse_quote!(1);

        verify(fields.resolve(indexed).is_err(), "named fields must reject tuple lookup")?;
        verify(pattern.contains("second"), "selected field must be bound")?;
        verify(!pattern.contains("first"), "unselected field must remain unbound")?;

        Ok(())
    }

    /// Tuple patterns preserve positional holes around selected fields.
    #[test]
    fn tuple_bindings_preserve_field_positions() -> syn::Result<()> {
        let input: DeriveInput = parse_quote!(
            struct Test(u8, u16, u32);
        );

        let Data::Struct(data) = input.data else {
            return Err(syn::Error::new(Span::call_site(), "test input must be a structure"));
        };

        let fields = Fields::from_syn(&data.fields)?;
        let indexed: FieldRef = parse_quote!(1);
        let second = fields.resolve(indexed)?;
        let pattern = FieldPattern(&fields, FieldSelection::Selected(from_ref(&second)))
            .expand()?
            .to_string();

        verify(pattern.contains("_1"), "selected tuple field must be bound")?;
        verify(pattern.starts_with('('), "tuple pattern must remain positional")?;

        Ok(())
    }
}
