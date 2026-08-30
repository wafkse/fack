//! `no_std` consumer fixture for the public `fack` facade.
//!
//! The fixture verifies that generated implementations require neither `std`
//! nor runtime allocation from the facade itself.
#![no_std]

use fack::prelude::*;

/// Minimal leaf error used by the fixture.
#[derive(Debug, Error)]
#[error("inner")]
pub struct Inner;

/// Error with a formatted field and an ordinary source.
#[derive(Debug, Error)]
#[error("outer {code:#x}")]
#[error(source(inner))]
// NOTE(invariant): Both private fields are the exact derive input exercised by
// this no_std fixture.
pub struct Outer {
    /// Numeric context rendered by the generated `Display` implementation.
    code: u8,

    /// Selected error source.
    inner: Inner,
}

/// Error with an optional tuple source.
#[derive(Debug, Error)]
#[error("optional")]
#[error(source(0))]
// NOTE(invariant): The private tuple field is the optional source exercised by
// this no_std fixture.
pub struct Optional(Option<Inner>);
