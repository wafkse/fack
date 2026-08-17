#![no_std]

use fack::prelude::*;

#[derive(Debug, Error)]
#[error("inner")]
pub struct Inner;

#[derive(Debug, Error)]
#[error("outer {code:#x}")]
#[error(source(inner))]
pub struct Outer {
    code: u8,
    inner: Inner,
}

#[derive(Debug, Error)]
#[error("optional")]
#[error(source(0))]
pub struct Optional(Option<Inner>);
