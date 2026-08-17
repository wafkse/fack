//! Error-source container semantics.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

use crate::{
    expand::Expand,
    field::{FieldId, Fields},
};

/// Supported source container shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceShape {
    /// A source stored directly as `E`.
    Direct,

    /// A source stored as `Box<E>`.
    Boxed,

    /// An optional source stored as `Option<E>`.
    Optional,

    /// An optional boxed source stored as `Option<Box<E>>`.
    OptionalBoxed,
}

/// A resolved source field and its semantic error type.
// NOTE(invariant): `field` belongs to the field collection used by `Source::new`.
// `error_type` is the error value after removing only the supported source
// containers represented by `shape`.
#[derive(Clone, Debug)]
pub struct Source {
    /// Resolved source field identity.
    field: FieldId,

    /// Supported container shape of the source field.
    shape: SourceShape,

    /// Semantic error type after removing supported containers.
    error_type: Type,
}

impl Source {
    /// Resolve source-container semantics for one field.
    #[must_use]
    #[inline]
    pub fn new(fields: &Fields, field: FieldId) -> Self {
        let field_type = fields.ty(field);
        let (shape, error_type) = SourceShape::inspect(field_type);

        Self { field, shape, error_type }
    }

    /// Return the resolved source field.
    #[inline]
    #[must_use]
    pub const fn field(&self) -> FieldId {
        let Self { field, .. } = self;

        *field
    }

    /// Return the source container shape.
    #[inline]
    #[must_use]
    pub const fn shape(&self) -> SourceShape {
        let Self { shape, .. } = self;

        *shape
    }

    /// Return the semantic error type inside the supported containers.
    #[inline]
    #[must_use]
    pub const fn error_type(&self) -> &Type {
        let Self { error_type, .. } = self;

        error_type
    }
}

impl SourceShape {
    /// Classify supported source containers and expose the semantic error type.
    fn inspect(field_type: &Type) -> (Self, Type) {
        if let Some(optional) = TypeContainer::single(field_type, "Option") {
            if let Some(boxed) = TypeContainer::single(optional, "Box") {
                return (Self::OptionalBoxed, boxed.clone());
            }

            return (Self::Optional, optional.clone());
        }

        if let Some(boxed) = TypeContainer::single(field_type, "Box") {
            return (Self::Boxed, boxed.clone());
        }

        (Self::Direct, field_type.clone())
    }
}

/// Inspection of one-argument source containers.
struct TypeContainer;

impl TypeContainer {
    /// Return the sole type argument when the target is the expected container.
    fn single<'a>(target: &'a Type, expected: &str) -> Option<&'a Type> {
        let Type::Path(path) = target else {
            return None;
        };
        let segment = path.path.segments.last()?;

        if segment.ident != expected {
            return None;
        }

        let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            return None;
        };
        let mut arguments = arguments.args.iter();
        let syn::GenericArgument::Type(inner) = arguments.next()? else {
            return None;
        };

        arguments.next().is_none().then_some(inner)
    }
}

/// Context required to expand one source expression.
#[derive(Clone, Debug)]
pub struct ExpandContext {
    /// Generated local bound to the source field.
    binding: Ident,

    /// Generated path root for error traits.
    root: TokenStream,

    /// Whether source chaining forwards through the selected error.
    transparent: bool,
}

impl ExpandContext {
    /// Construct a source expansion context.
    #[inline]
    #[must_use]
    pub const fn new(binding: Ident, root: TokenStream, transparent: bool) -> Self {
        Self {
            binding,
            root,
            transparent,
        }
    }
}

impl Expand for &Source {
    /// Binding and root-path information required to emit the source
    /// expression.
    type Context = ExpandContext;

    /// Generate ordinary or transparent source-chain behavior from resolved
    /// shape.
    #[inline]
    fn expand_with(self, context: Self::Context) -> syn::Result<TokenStream> {
        let ExpandContext {
            binding,
            root,
            transparent,
        } = context;

        if transparent {
            return match self.shape() {
                SourceShape::Direct => Ok(quote! { #root::error::Error::source(#binding) }),
                SourceShape::Boxed => Ok(quote! { #root::error::Error::source(&**#binding) }),
                SourceShape::Optional | SourceShape::OptionalBoxed => Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "optional transparent source reached expansion",
                )),
            };
        }

        match self.shape() {
            SourceShape::Direct => Ok(quote! { Some(#binding) }),
            SourceShape::Boxed => Ok(quote! { Some(&**#binding) }),
            SourceShape::Optional => Ok(quote! {
                #binding.as_ref().map(|source| source as &(dyn #root::error::Error + 'static))
            }),
            SourceShape::OptionalBoxed => Ok(quote! {
                #binding.as_ref().map(|source| &**source as &(dyn #root::error::Error + 'static))
            }),
        }
    }
}
