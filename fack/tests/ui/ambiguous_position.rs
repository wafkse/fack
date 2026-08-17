use fack::prelude::*;
#[derive(Debug, Error)]
#[error("outer {0}", 3)]
struct Error(u8);
fn main() {}
