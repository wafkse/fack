//! Final code expansion for error definitions.

use alloc::{format, vec::Vec};

use proc_macro2::TokenStream;

use quote::{quote, ToTokens};

use syn::Ident;

use super::{
    common::{FieldRef, Format, ImportRoot, InlineOptions, Transparent},
    enumerate::{Enumeration, Variant},
    structure::{Structure, StructureOptions},
    Target,
};

/// A struct that encapsulates the code generation for error definitions.
pub struct Expand {}

impl Expand {
    /// Expand the [`Target`] into the appropriate error implementation.
    #[inline]
    pub fn target(target_value: Target) -> syn::Result<TokenStream> {
        match target_value {
            Target::Struct(structure) => Self::structure(structure),
            Target::Enum(enumeration) => Self::enumeration(enumeration),
        }
    }
}

impl Expand {
    /// Expand to the error implementation for the target [`Structure`].
    pub fn structure(target_value: Structure) -> syn::Result<TokenStream> {
        let Structure {
            inline_opts,
            root_import,
            name_ident,
            generics,
            options,
            field_list,
        } = target_value;

        let inline_expand = match inline_opts {
            Some(InlineOptions::Neutral) => quote! { #[inline] },
            Some(InlineOptions::Always) => quote! { #[inline(always)] },
            Some(InlineOptions::Never) => quote! { #[inline(never)] },
            None => quote! {},
        };

        let root_expand = root_import.map_or_else(|| quote! { ::core }, |ImportRoot(root)| root.to_token_stream());

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let field_pat = field_list.pattern()?;

        match options {
            StructureOptions::Standalone {
                source_field,
                format_args: Format { format, format_args },
            } => {
                let source_expand = if let Some(field) = source_field { quote! { Some(&self.#field) } } else { quote! { None } };

                Ok(quote! {
                    #[automatically_derived]
                    impl #impl_generics #root_expand::error::Error for #name_ident #ty_generics #where_clause {
                        #inline_expand
                        fn source(&self) -> Option<&(dyn #root_expand::error::Error + 'static)> {
                            #source_expand
                        }
                    }

                    #[automatically_derived]
                    impl #impl_generics #root_expand::fmt::Display for #name_ident #ty_generics #where_clause {
                        #inline_expand
                        fn fmt(&self, f: &mut #root_expand::fmt::Formatter<'_>) -> #root_expand::fmt::Result {
                            let &Self #field_pat = self;

                            #root_expand::write!(f, #format, #format_args)
                        }
                    }
                })
            }
            StructureOptions::Transparent(Transparent(target_field)) => {
                let field_expand = quote! {
                    self.#target_field
                };

                Ok(quote! {
                    #[automatically_derived]
                    impl #impl_generics #root_expand::error::Error for #name_ident #ty_generics #where_clause {
                        #inline_expand
                        fn source(&self) -> Option<&(dyn #root_expand::error::Error + 'static)> {
                            #root_expand::error::Error::source(&#field_expand)
                        }
                    }

                    #[automatically_derived]
                    impl #impl_generics #root_expand::fmt::Display for #name_ident #ty_generics #where_clause {
                        #inline_expand
                        fn fmt(&self, f: &mut #root_expand::fmt::Formatter<'_>) -> #root_expand::fmt::Result {
                            let &Self #field_pat = self;

                            #root_expand::fmt::Display::fmt(&#field_expand, f)
                        }
                    }
                })
            }
            StructureOptions::Forward {
                field_ref,
                field_type,
                source_field,
                format_args: Format { format, format_args },
            } => {
                let source_expand = if let Some(field) = source_field { quote! { Some(&self.#field) } } else { quote! { None } };

                let from_expand = match field_ref {
                    FieldRef::Named(ref field) => quote! {
                        #[automatically_derived]
                        impl #impl_generics From<#field_type> for #name_ident #ty_generics #where_clause {
                            #inline_expand
                            fn from(#field: #field_type) -> Self {
                               Self { #field }
                            }
                        }
                    },
                    FieldRef::Indexed(..) => quote! {
                        #[automatically_derived]
                        impl #impl_generics From<#field_type> for #name_ident #ty_generics #where_clause {
                            #inline_expand
                            fn from(field: #field_type) -> Self {
                                Self(field)
                            }
                        }
                    },
                };

                Ok(quote! {
                    #from_expand

                    #[automatically_derived]
                    impl #impl_generics #root_expand::error::Error for #name_ident #ty_generics #where_clause {
                        #inline_expand
                        fn source(&self) -> Option<&(dyn #root_expand::error::Error + 'static)> {
                            #source_expand
                        }
                    }

                    #[automatically_derived]
                    impl #impl_generics #root_expand::fmt::Display for #name_ident #ty_generics #where_clause {
                        #inline_expand
                        fn fmt(&self, f: &mut #root_expand::fmt::Formatter<'_>) -> #root_expand::fmt::Result {
                            let &Self #field_pat = self;

                            #root_expand::write!(f, #format, #format_args)
                        }
                    }
                })
            }
        }
    }

    /// Expand to the error implementation for the target [`Enumeration`].
    pub fn enumeration(target_value: Enumeration) -> syn::Result<TokenStream> {
        let Enumeration {
            inline_opts,
            root_import,
            name_ident,
            generics,
            variant_list,
        } = target_value;

        let inline_expand = match inline_opts {
            Some(InlineOptions::Neutral) => quote! { #[inline] },
            Some(InlineOptions::Always) => quote! { #[inline(always)] },
            Some(InlineOptions::Never) => quote! { #[inline(never)] },
            None => quote! {},
        };

        let root_expand = root_import.map_or_else(|| quote! { ::core }, |ImportRoot(root)| root.to_token_stream());

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let mut source_expand = Vec::new();

        let mut from_expand = Vec::new();

        let mut display_expand = Vec::new();

        for variant in variant_list {
            match variant {
                Variant::Struct {
                    format: Format { format, format_args },
                    variant_name,
                    field_list,
                    source_field,
                } => {
                    let field_pat = field_list.pattern()?;

                    if let Some(source_field) = source_field {
                        let field_expand = match source_field {
                            FieldRef::Named(ref field) => quote! { self.#field },
                            FieldRef::Indexed(index) => Ident::new(&format!("_{index}"), variant_name.span()).into_token_stream(),
                        };

                        source_expand.push(quote! {
                            &Self::#variant_name #field_pat => Some(#field_expand),
                        });
                    } else {
                        source_expand.push(quote! {
                            &Self::#variant_name #field_pat => None,
                        });
                    }

                    display_expand.push(quote! {
                        &Self::#variant_name #field_pat  => #root_expand::write!(f, #format, #format_args),
                    });
                }
                Variant::Unit {
                    format: Format { format, format_args },
                    variant_name,
                } => {
                    source_expand.push(quote! {
                        &Self::#variant_name => None,
                    });

                    display_expand.push(quote! {
                        &Self::#variant_name => #root_expand::write!(f, #format, #format_args),
                    });
                }
                Variant::Transparent {
                    transparent: Transparent(trans_field),
                    variant_name,
                    field_list,
                } => {
                    let trans_expand = match trans_field {
                        FieldRef::Named(ref field) => quote! { #field },
                        FieldRef::Indexed(index) => Ident::new(&format!("_{index}"), variant_name.span()).into_token_stream(),
                    };

                    let field_pat = field_list.pattern()?;

                    source_expand.push(quote! {
                        &Self::#variant_name #field_pat => #root_expand::error::Error::source(#trans_expand),
                    });

                    display_expand.push(quote! {
                        &Self::#variant_name #field_pat => #root_expand::fmt::Display::fmt(#trans_expand, f),
                    });
                }
                Variant::Forward {
                    format: Format { format, format_args },
                    variant_name,
                    field_ref,
                    field_type,
                } => {
                    let bare_field_name = quote!(#field_ref);

                    source_expand.push(quote! {
                        &Self::#variant_name { #bare_field_name: ref target_field }  => #root_expand::error::Error::source(target_field),
                    });

                    from_expand.push(match field_ref {
                        FieldRef::Named(ref field) => quote! {
                            #[automatically_derived]
                            impl #impl_generics From<#field_type> for #name_ident #ty_generics #where_clause {
                                #inline_expand
                                fn from(#field: #field_type) -> Self {
                                    Self::#variant_name { #field }
                                }
                            }
                        },
                        FieldRef::Indexed(..) => quote! {
                            #[automatically_derived]
                            impl #impl_generics From<#field_type> for #name_ident #ty_generics #where_clause {
                                #inline_expand
                                fn from(field: #field_type) -> Self {
                                    Self::#variant_name(field)
                                }
                            }
                        },
                    });

                    let replaced_field_name = match field_ref {
                        FieldRef::Named(..) => None,
                        FieldRef::Indexed(ref field) => Some({
                            let ident = Ident::new(&format!("_{field}"), variant_name.span());

                            quote! {
                                : ref #ident
                            }
                        }),
                    };

                    display_expand.push(quote! {
                        &Self::#variant_name { #bare_field_name #replaced_field_name }  => #root_expand::write!(f, #format, #format_args),
                    });
                }
            }
        }

        Ok(quote! {
            #(#from_expand)*

            #[automatically_derived]
            impl #impl_generics #root_expand::error::Error for #name_ident #ty_generics #where_clause {
                #inline_expand
                fn source(&self) -> Option<&(dyn #root_expand::error::Error + 'static)> {
                    match self {
                        #(#source_expand)*
                    }
                }
            }

            #[automatically_derived]
            impl #impl_generics #root_expand::fmt::Display for #name_ident #ty_generics #where_clause {
                #inline_expand
                fn fmt(&self, f: &mut #root_expand::fmt::Formatter<'_>) -> #root_expand::fmt::Result {
                    match self {
                        #(#display_expand)*
                    }
                }
            }
        })
    }
}
