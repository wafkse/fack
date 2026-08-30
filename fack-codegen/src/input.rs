//! Parsing and syntax level classification for derive input.
//!
//! The input subsystem owns source syntax, structural fields, diagnostics, and
//! target classification. Its crate-internal handoff contains only declarations
//! that are syntactically coherent but may still require field resolution.

mod attribute;
mod diagnostic;
mod field;
mod format;
mod target;

pub use attribute::{Declaration, Display, ImportRoot, InlinePolicy, Source};
pub use field::{Field, FieldRef, Fields};
pub use format::{Argument, Format};
pub use target::{Enumeration, Structure, Target, Variant};
