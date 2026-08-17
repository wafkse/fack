use fack::prelude::*;
#[derive(Debug, Error)]
#[error("outer")]
#[error(from)]
struct Error(std::io::Error, u8);
fn main() {}
