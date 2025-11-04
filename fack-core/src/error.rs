//! A module for the [`Error`] trait and its implementations.
//!
//! See the [`Error`] trait for more information.

use core::error::Error as CoreError;

/// A trait for types that can be used to represent an error.
///
/// Particularly, this is a supertrait of the core library [`Error`] trait.
///
/// [`Error`]: CoreError
pub trait Error: CoreError {}

/// Blanket implementation of [`Error`] for types that are also core
/// [`Error`](CoreError)s.
impl<E> Error for E where E: CoreError {}
