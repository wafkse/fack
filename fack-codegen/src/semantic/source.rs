//! Error source container semantics.
//!
//! Source resolution consumes an owned resolved field proof and classifies its
//! Rust type into the supported direct, boxed, and optional containers. The
//! transparent source type excludes optional containers by construction.

use syn::Type;

use crate::input::Field;

/// One resolved source field and its semantic error type.
#[derive(Clone, Debug)]
// NOTE(invariant): The resolved field and semantic error type describe the same
// source after supported container removal.
pub struct ErrorField {
    /// Resolved source field proof.
    field: Field,

    /// Semantic error type after supported container removal.
    error_type: Type,
}

impl ErrorField {
    /// Return the resolved source field.
    #[inline]
    #[must_use]
    pub const fn field(&self) -> &Field {
        let &Self { ref field, .. } = self;

        field
    }

    /// Return the semantic error type.
    #[inline]
    #[must_use]
    pub const fn error_type(&self) -> &Type {
        let &Self { ref error_type, .. } = self;

        error_type
    }
}

/// Supported ordinary source container states.
#[derive(Clone, Debug)]
pub enum Source {
    /// A source stored directly as `E`.
    Direct(ErrorField),

    /// A source stored as `Box<E>`.
    Boxed(ErrorField),

    /// A source stored as `Option<E>`.
    Optional(ErrorField),

    /// A source stored as `Option<Box<E>>`.
    OptionalBoxed(ErrorField),
}

impl Source {
    /// Resolve source container semantics for one field proof.
    #[must_use]
    #[inline]
    pub fn new(field: Field) -> Self {
        let field_type = field.ty();

        let (shape, error_type) = match TypeContainer::single(field_type, "Option") {
            Some(optional) => match TypeContainer::single(optional, "Box") {
                Some(boxed) => (SourceShape::OptionalBoxed, boxed.clone()),
                None => (SourceShape::Optional, optional.clone()),
            },
            None => match TypeContainer::single(field_type, "Box") {
                Some(boxed) => (SourceShape::Boxed, boxed.clone()),
                None => (SourceShape::Direct, field_type.clone()),
            },
        };

        let field = ErrorField { field, error_type };

        match shape {
            SourceShape::Direct => Self::Direct(field),
            SourceShape::Boxed => Self::Boxed(field),
            SourceShape::Optional => Self::Optional(field),
            SourceShape::OptionalBoxed => Self::OptionalBoxed(field),
        }
    }

    /// Return the resolved source field.
    #[inline]
    #[must_use]
    pub const fn field(&self) -> &Field {
        match self {
            &Self::Direct(ref field) | &Self::Boxed(ref field) | &Self::Optional(ref field) | &Self::OptionalBoxed(ref field) => {
                field.field()
            }
        }
    }

    /// Return the semantic error type.
    #[inline]
    #[must_use]
    pub const fn error_type(&self) -> &Type {
        match self {
            &Self::Direct(ref field) | &Self::Boxed(ref field) | &Self::Optional(ref field) | &Self::OptionalBoxed(ref field) => {
                field.error_type()
            }
        }
    }

    /// Convert this source into a transparent source when its shape permits it.
    #[inline]
    pub fn into_transparent(self) -> Option<TransparentSource> {
        match self {
            Self::Direct(field) => Some(TransparentSource::Direct(field)),
            Self::Boxed(field) => Some(TransparentSource::Boxed(field)),
            Self::Optional(_) | Self::OptionalBoxed(_) => None,
        }
    }
}

/// Source states that are valid for transparent forwarding.
#[derive(Clone, Debug)]
pub enum TransparentSource {
    /// A transparent source stored directly as `E`.
    Direct(ErrorField),

    /// A transparent source stored as `Box<E>`.
    Boxed(ErrorField),
}

impl TransparentSource {
    /// Return the resolved source field.
    #[inline]
    #[must_use]
    pub const fn field(&self) -> &Field {
        match self {
            &Self::Direct(ref field) | &Self::Boxed(ref field) => field.field(),
        }
    }

    /// Return the semantic error type.
    #[inline]
    #[must_use]
    pub const fn error_type(&self) -> &Type {
        match self {
            &Self::Direct(ref field) | &Self::Boxed(ref field) => field.error_type(),
        }
    }
}

/// Supported source container classification before semantic construction.
enum SourceShape {
    /// Direct error value.
    Direct,

    /// Boxed error value.
    Boxed,

    /// Optional direct error value.
    Optional,

    /// Optional boxed error value.
    OptionalBoxed,
}

/// Inspection of one argument source containers.
struct TypeContainer;

impl TypeContainer {
    /// Return the sole type argument for the expected container.
    fn single<'a>(target: &'a Type, expected: &str) -> Option<&'a Type> {
        let path = match target {
            &Type::Path(ref path) => Some(path),
            _ => None,
        }?;
        let segment = path.path.segments.last()?;
        let arguments = match segment.ident == expected {
            true => Some(&segment.arguments),
            false => None,
        }?;
        let arguments = match arguments {
            &syn::PathArguments::AngleBracketed(ref arguments) => Some(arguments),
            _ => None,
        }?;
        let mut values = arguments.args.iter();
        let inner = match values.next()? {
            &syn::GenericArgument::Type(ref inner) => Some(inner),
            _ => None,
        }?;

        values.next().is_none().then_some(inner)
    }
}
