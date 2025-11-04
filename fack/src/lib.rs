#![no_std]
#![doc = include_str!("../../README.md")]
#![forbid(unsafe_code, missing_docs, rustdoc::all, clippy::all, clippy::pedantic)]

pub mod prelude {
    //! Re-exports the most commonly used traits and types.

    pub use fack_core::prelude::*;
}
