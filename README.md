# fack

[![CI](https://github.com/wafkse/fack/actions/workflows/ci.yml/badge.svg?branch=trunk)](https://github.com/wafkse/fack/actions/workflows/ci.yml)

Error handling derive macro for Rust. `no_std` compatible, doesn't allocate.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("file not found: {path}")]
struct FileError {
    path: String,
}
```

## Installation

```toml
[dependencies]
fack = "0.1.0"
```

## What's different from thiserror?

- Actually runs in `no_std` environments - uses `::core` by default, zero heap allocations at runtime
- Control inlining with `#[error(inline(...))]` - matters for hot paths and code size
- `fack-codegen` is a standalone library - use it in your own macros or build scripts
- Preserves source spans properly for better IDE integration

If you're writing embedded code or care about allocation-free error handling, this might be useful.

## Examples

Basic struct:

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("database error: {msg}")]
struct DbError {
    msg: String,
}
```

Error chaining:

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

Enums with variants:

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
enum ParseError {
    #[error("invalid syntax at line {line}")]
    Syntax { line: usize },

    #[error("unexpected end of file")]
    Eof,

    #[error(transparent(0))]
    Io(std::io::Error),
}
```

Auto-conversion with `from`:

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

// Now you can use ? with these error types
fn example() -> Result<(), AppError> {
    let _file = std::fs::read("file.txt")?;  // converts io::Error
    let _num: i32 = "123".parse()?;          // converts ParseIntError
    Ok(())
}
```

## Attributes

**Format strings** - `#[error("message")]`

Use `{field}` for named fields, `{_0}` for tuple fields. Standard format specifiers work.

```rust,no_run
#[derive(Error, Debug)]
#[error("value {value} out of range {min}..{max}")]
struct RangeError { value: i32, min: i32, max: i32 }

#[derive(Error, Debug)]
#[error("invalid input: {_0:?}")]
struct InputError(String);
```

**Source** - `#[error(source(field))]`

Mark which field contains the underlying error. Enables error chain traversal.

```rust,no_run
#[derive(Error, Debug)]
#[error("operation failed")]
#[error(source(cause))]
struct OpError {
    cause: std::io::Error,
}
```

**Transparent** - `#[error(transparent(field))]`

Forward display and source to an inner error. Useful for wrapper types.

```rust,no_run
#[derive(Error, Debug)]
#[error(transparent(0))]
struct Wrapper(std::io::Error);
```

**From** - `#[error(from)]`

Generate `From<T>` impl for the error type. Requires exactly one field.

```rust,no_run
#[derive(Error, Debug)]
#[error("wrapped io error")]
#[error(from)]
struct IoWrapper(std::io::Error);
```

**Inline** - `#[error(inline(strategy))]`

Control inlining of generated methods. Options: `neutral` (default), `always`, `never`.

```rust,no_run
#[derive(Error, Debug)]
#[error(inline(always))]  // force inline for hot paths
#[error("fast error")]
struct HotPathError { code: u32 }
```

**Import** - `#[error(import(path))]`

Override the default `::core` import. Use `::std` if you need std-specific features.

```rust,no_run
#[derive(Error, Debug)]
#[error(import(::std))]
#[error("std error")]
struct StdError { msg: String }
```

## Documentation

Full docs at [docs.rs/fack](https://docs.rs/fack).

## Workspace structure

This repo has four crates:

- `fack` - Main crate, re-exports everything
- `fack-core` - Error trait definition
- `fack-macro` - Procedural macro implementation
- `fack-codegen` - Code generation engine

The `fack-codegen` crate is deliberately not a proc-macro crate. You can depend on it in regular code to build custom error macros or generate errors at build time.

## License

GPL-3.0

Copyright (C) 2025 W. Frakchi

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

See [LICENSE.md](LICENSE.md) for the full text.
