use fack::prelude::*;
#[derive(Debug, Error)]
#[error("outer")]
#[error(source(inner))]
#[error(source(inner))]
struct Error { inner: std::io::Error }
fn main() {}
