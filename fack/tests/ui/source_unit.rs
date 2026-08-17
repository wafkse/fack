use fack::prelude::*;
#[derive(Debug, Error)]
#[error("outer")]
#[error(source(0))]
struct Error;
fn main() {}
