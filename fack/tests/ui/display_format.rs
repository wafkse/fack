use fack::prelude::*;
fn custom(_: &Error, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
#[derive(Debug, Error)]
#[error("outer")]
#[error(display(custom))]
struct Error;
fn main() {}
