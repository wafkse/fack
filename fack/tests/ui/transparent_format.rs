use fack::prelude::*;
#[derive(Debug, Error)]
#[error("outer")]
#[error(transparent(0))]
struct Error(std::io::Error);
fn main() {}
