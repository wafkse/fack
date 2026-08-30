use fack::prelude::*;

#[derive(Debug, Error)]
#[error("outer")]
#[error(source(2))]
struct Error(std::io::Error);

fn main() {}
