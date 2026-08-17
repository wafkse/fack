use fack::prelude::*;
#[derive(Debug, Error)]
#[error(transparent(0))]
struct Error(Option<std::io::Error>);
fn main() {}
