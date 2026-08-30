use fack::prelude::*;

#[derive(Debug, Error)]
#[error("outer")]
#[error(from)]
struct Error;

fn main() {}
