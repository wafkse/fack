use fack::prelude::*;
#[derive(Debug, Error)]
#[error(transparent(0))]
#[error(from)]
struct Error(std::io::Error);
fn main() {}
