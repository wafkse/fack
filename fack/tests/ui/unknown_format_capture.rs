use fack::prelude::*;
#[derive(Debug, Error)]
#[error("outer {missing}")]
struct Error { present: u8 }
fn main() {}
