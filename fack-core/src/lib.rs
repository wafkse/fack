//! Core *Fack* utilities for error encapsulation and handling.
//!
//! This module provides the generator code for [`Error`] implementations.
//!
//! [`Error`]: crate::error::Error
#![no_std]
#![forbid(unsafe_code, missing_docs, rustdoc::all, clippy::all, clippy::pedantic)]

extern crate alloc;

pub mod error;

pub mod prelude {
    //! A module that re-exports the most commonly used types and traits in this
    //! crate.

    pub use crate::error::Error;
}
