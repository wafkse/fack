use fack::prelude::*;
#[derive(Debug, Error)]
#[error("outer")]
#[error(source(0))]
struct Error { inner: std::io::Error }
fn main() {}
