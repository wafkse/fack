# fack

[![CI](https://github.com/wafkse/fack/actions/workflows/ci.yml/badge.svg?branch=trunk)](https://github.com/wafkse/fack/actions/workflows/ci.yml)

Declarative Rust error derivation with `no_std` support and zero allocation in generated runtime code.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("file not found {path}")]
struct FileError {
    path: String,
}
```

## Installation

```toml
[dependencies]
fack = "0.2.0"
```

## Design

- All derive behavior uses the single `#[error(...)]` helper namespace
- Generated implementations use `::core` unless an import root is selected
- Format fields and source fields are resolved before expansion
- `fack-codegen` exposes opaque parse, validate, and expansion stages for downstream tooling
- Generated `Display`, `Error`, and `From` implementations allocate nothing

## Formatting

Named and tuple fields can be captured directly. Explicit Rust format arguments are also supported.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("invalid input {input:?}")]
struct InputError {
    input: String,
}

#[derive(Error, Debug)]
#[error("invalid value {0}")]
struct ValueError(i32);

#[derive(Error, Debug)]
#[error("normalized {value}", value = input.trim())]
struct NormalizedError {
    input: String,
}
```

A custom formatter can be selected when a format string is not the right abstraction.

```rust,no_run
use fack::prelude::*;

fn render(error: &RenderedError, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let RenderedError { code } = error;
    write!(f, "rendered {code}")
}

#[derive(Error, Debug)]
#[error(display(render))]
struct RenderedError {
    code: u8,
}
```

## Sources

`#[error(source(field))]` exposes the selected field as the ordinary source. Direct, boxed, optional, and optional boxed sources are supported.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("request failed")]
#[error(source(io))]
struct RequestError {
    io: std::io::Error,
    url: String,
}
```

`#[error(transparent(field))]` requires exactly one non-optional field. Display forwards to that field and source chaining forwards through its own `Error::source` implementation.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error(transparent(0))]
struct Wrapper(std::io::Error);
```

## Conversion

`#[error(from)]` requires exactly one field. It generates `From<T>` and makes that field the ordinary error source.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
enum AppError {
    #[error("io error")]
    #[error(from)]
    Io(std::io::Error),

    #[error("parse error")]
    #[error(from)]
    Parse(std::num::ParseIntError),
}
```

## Generation options

Without an inline declaration, generated methods receive no explicit inline attribute. `#[error(inline)]` and `#[error(inline(neutral))]` emit ordinary `#[inline]`. The `always` and `never` strategies emit their corresponding Rust attributes.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error(inline(never))]
#[error("rare error")]
struct RareError;
```

Generated paths use `::core` by default. `#[error(import(path))]` selects a different root.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error(import(::std))]
#[error("standard error")]
struct StdError;
```

Enum-wide generation options belong on the enum. Display, source, transparency, and conversion declarations belong on variants.

## Code generation engine

`fack-codegen` is a regular `no_std` library rather than a proc-macro crate. Its supported API keeps compiler representations opaque while allowing downstream tools to choose staged or one-shot generation.

```rust,ignore
let target = fack_codegen::Target::input(&input)?;
let validated = target.validate()?;
let tokens = validated.expand()?;

let tokens = fack_codegen::generate(&input)?;
```

## Workspace

- `fack` provides the user-facing facade
- `fack-core` provides the blanket error capability
- `fack-macro` provides the derive macro entry point
- `fack-codegen` provides the staged generation engine

## License

GPL-3.0

Copyright (C) 2025 W. Frakchi

This program is free software. You can redistribute it and modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License or any later version.

See [LICENSE.md](LICENSE.md) for the full text.
