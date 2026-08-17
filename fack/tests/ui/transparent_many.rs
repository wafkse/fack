use fack::prelude::*;
#[derive(Debug, Error)]
#[error(transparent(0))]
struct Error(std::io::Error, u8);
fn main() {}
