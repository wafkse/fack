use fack::prelude::*;
#[derive(Debug, Error)]
#[error("outer")]
#[error(source(missing))]
struct Error { inner: std::io::Error }
fn main() {}
