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
fack = "0.4.0"
```

Import the prelude to bring both the derive macro and `core::error::Error` into scope.

```rust,no_run
use fack::prelude::*;
```

## Formatting

Use a format string to implement `Display`.

Named fields can be captured directly.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("invalid input {input:?}")]
struct InputError {
    input: String,
}
```

Tuple fields use positional captures.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("invalid value {0}")]
struct ValueError(i32);
```

Explicit Rust expressions are accepted as format arguments.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("normalized {value}", value = input.trim())]
struct NormalizedError {
    input: String,
}
```

Positional expressions are also supported.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("absolute value {}", value.abs())]
struct AbsoluteError {
    value: i32,
}
```

Dynamic width and precision may reference fields through normal Rust format syntax.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("value {value:>width$}")]
struct PaddedError {
    value: u32,
    width: usize,
}
```

## Custom display

Use `display(path)` when formatting belongs in a formatter function.

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

## Error sources

Use `source(field)` to expose an ordinary error source.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("request failed for {url}")]
#[error(source(io))]
struct RequestError {
    io: std::io::Error,
    url: String,
}
```

Direct, boxed, optional, and optional boxed sources are supported.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("optional failure")]
#[error(source(source))]
struct OptionalError {
    source: Option<Box<std::io::Error>>,
}
```

## Transparent errors

Use `transparent(field)` to forward `Display` and source chaining through one non-optional error field.

Additional fields may retain context.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error(transparent(inner))]
struct Wrapper {
    context: u8,
    inner: std::io::Error,
}
```

## Conversion with `From`

Use `from` on a declaration with exactly one field. It generates `From<T>` and uses that field as the ordinary source.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("parse failed")]
#[error(from)]
struct ParseError(std::num::ParseIntError);
```

The same form works on enum variants.

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

## Enum errors

Each variant declares its own display, source, transparency, or conversion behavior.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
enum LoadError {
    #[error("missing file {0}")]
    Missing(String),

    #[error("failed to read {path}")]
    #[error(source(source))]
    Read {
        path: String,
        source: std::io::Error,
    },

    #[error(transparent(source))]
    Io {
        source: std::io::Error,
        attempts: u8,
    },
}
```

## Generation options

Without an inline declaration, generated methods receive no explicit inline attribute.

`#[error(inline)]` and `#[error(inline(neutral))]` emit ordinary `#[inline]`. The `always` and `never` strategies emit the corresponding Rust attributes.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error(inline(never))]
#[error("rare error")]
struct RareError;
```

Generated paths use `::core` by default. Use `import(path)` to select another root.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error(import(::std))]
#[error("standard error")]
struct StdError;
```

Enum-wide generation options belong on the enum.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error(inline)]
#[error(import(::std))]
enum ConfiguredError {
    #[error("first error")]
    First,

    #[error("second error")]
    Second,
}
```

## `no_std`

Generated implementations use `core` by default, so the facade can be used from `no_std` crates without switching import roots.

```rust,ignore
#![no_std]

use fack::prelude::*;

#[derive(Error, Debug)]
#[error("device error {code}")]
struct DeviceError {
    code: u8,
}
```

## Associated crates

- `fack` provides the user-facing facade
- `fack-core` reserves the foundational `no_std` API boundary
- `fack-macro` provides the derive macro
- `fack-codegen` provides the standalone Syn 3 code generation API

## License

GPL-3.0

Copyright (C) 2025 W. Frakchi

This program is free software. You can redistribute it and modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License or any later version.

See [LICENSE.md](LICENSE.md) for the full text.
