# fack

A declarative error handling library for Rust that eliminates boilerplate through intelligent macro-driven code generation.

## Overview

`fack` provides a `#[derive(Error)]` macro that generates complete `Error` and `Display` implementations from declarative attributes. The library is architected for maximum compatibility: `no_std` by default, zero runtime allocations, and composable code generation primitives.

<br>

## Architecture

The workspace comprises four specialized crates:

<br>

- **fack**: Unified facade providing the complete error handling API
- **fack-core**: Trait definitions with `no_std` compatibility, zero dependencies
- **fack-macro**: Procedural macro entry point for `#[derive(Error)]`
- **fack-codegen**: Standalone code generation engine, usable outside proc-macro contexts

### Design Principles

**no_std First**: All crates operate without the standard library. Generated code uses `::core` by default, with opt-in `::std` support.

<br>

**no_alloc Runtime**: While code generation uses `alloc` for internal data structures, the generated error implementations require no heap allocation during normal operation.

<br>

**Composable Code Generation**: `fack-codegen` is deliberately not a proc-macro crate, enabling direct integration into custom build systems, code generators, or other procedural macros.

<br>

**Span Preservation**: The macro preserves source spans from the original code, ensuring error messages, IDE hints, and language server features point to the correct locations in your source files. This provides an unparalleled LSP experience with accurate go-to-definition, hover information, and diagnostics.

---

## Usage

```toml
[dependencies]
fack = "0.1.0"
```

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("configuration error: {message}")]
struct ConfigError {
    message: String,
}
```

---

## Attribute Reference

The `#[error(...)]` attribute controls all aspects of error behavior. Multiple attributes can be applied to a single type or variant, each serving a distinct purpose.

<br>

### Format String: `#[error("...")]`

A `Display` implementation is generated for your error if you provide `#[error("...")]` messages on the struct or each variant of your enum.

<br>

The messages support a shorthand for interpolating fields from the error:

- `#[error("{var}")]` ⟶ `write!("{}", self.var)`
- `#[error("{0}")]` ⟶ `write!("{}", self.0)`
- `#[error("{var:?}")]` ⟶ `write!("{:?}", self.var)`
- `#[error("{0:?}")]` ⟶ `write!("{:?}", self.0)`

<br>

These shorthands can be used together with any additional format arguments, which may be arbitrary expressions. For example:

```rust,no_run
#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid rdo_lookahead_frames {0} (expected < {max})", max = i32::MAX)]
    InvalidLookahead(u32),
}
```

<br>

If one of the additional expression arguments needs to refer to a field of the struct or enum, then refer to named fields as `.var` and tuple fields as `.0`:

```rust,no_run
#[derive(Error, Debug)]
pub enum Error {
    #[error("first letter must be lowercase but was {:?}", first_char(.0))]
    WrongCase(String),

    #[error("invalid index {idx}, expected at least {} and at most {}", .limits.lo, .limits.hi)]
    OutOfBounds { idx: usize, limits: Limits },
}
```

<br>

All format arguments are evaluated in the context of the error type, with fields automatically destructured for access.

<br>

### Source Declaration: `#[error(source(field))]`

The Error trait's `source()` method is implemented to return whichever field has a `#[error(source(...))]` attribute, if any. This is for identifying the underlying lower-level error that caused your error.

<br>

Any error type that implements `std::error::Error` or dereferences to `dyn std::error::Error` will work as a source.

<br>

```rust,no_run
#[derive(Error, Debug)]
pub struct MyError {
    msg: String,
    #[error(source(cause))]
    cause: std::io::Error,
}
```

<br>

The source field can be referenced by name for named fields:

```rust,no_run
#[derive(Error, Debug)]
#[error("network request failed")]
#[error(source(io_error))]
struct NetworkError {
    io_error: std::io::Error,
    url: String,
}
```

<br>

Or by index for tuple struct fields:

```rust,no_run
#[derive(Error, Debug)]
#[error("wrapped error")]
#[error(source(0))]
struct Wrapper(std::io::Error, String);
```

The `source()` method returns `Option<&(dyn std::error::Error + 'static)>`, enabling error chain traversal for debugging and logging purposes.

### Transparent Forwarding: `#[error(transparent(field))]`

Errors may use `#[error(transparent(field))]` to forward the source and Display methods straight through to an underlying error without adding an additional message. This would be appropriate for enums that need an "anything else" variant:

<br>

```rust,no_run
#[derive(Error, Debug)]
pub enum MyError {
    #[error("something went wrong")]
    SomethingBad,

    #[error(transparent(0))]
    Other(anyhow::Error),  // source and Display delegate to anyhow::Error
}
```

<br>

Another use case is hiding implementation details of an error representation behind an opaque error type, so that the representation is able to evolve without breaking the crate's public API:

```rust,no_run
// PublicError is public, but opaque and easy to keep compatible.
#[derive(Error, Debug)]
#[error(transparent(inner))]
pub struct PublicError {
    inner: ErrorRepr,
}

impl PublicError {
    // Accessors for anything we do want to expose publicly.
}

// Private and free to change across minor versions of the crate.
#[derive(Error, Debug)]
enum ErrorRepr {
    // ...
}
```

<br>

Transparent errors behave identically to their wrapped type for both display and error chaining purposes. The specified field must implement `std::error::Error`.

<br>

### Automatic Conversion: `#[error(from)]`

A `From` implementation is generated for each variant or struct that contains an `#[error(from)]` attribute. This enables automatic error conversion using the `?` operator.

<br>

The variant or struct using `#[error(from)]` must contain exactly one field. The field may be named or unnamed:

<br>

```rust,no_run
#[derive(Error, Debug)]
pub enum MyError {
    #[error("io error")]
    #[error(from)]
    Io(std::io::Error),

    #[error("parse error")]
    #[error(from)]
    Parse(std::num::ParseIntError),
}
```

<br>

This generates:

```rust,no_run
impl From<std::io::Error> for MyError {
    fn from(source: std::io::Error) -> Self {
        MyError::Io(source)
    }
}

impl From<std::num::ParseIntError> for MyError {
    fn from(source: std::num::ParseIntError) -> Self {
        MyError::Parse(source)
    }
}
```

<br>

Named field example:

```rust,no_run
#[derive(Error, Debug)]
#[error("io operation failed")]
#[error(from)]
struct IoError {
    inner: std::io::Error,
}
```

<br>

The `#[error(from)]` attribute always implies that the same field is also the error source, so you don't need to specify `#[error(source(...))]` separately when using `#[error(from)]`.

<br>

### Inline Control: `#[error(inline(...))]`

Controls the inlining behavior of generated trait implementations. This attribute allows fine-tuning performance characteristics of error handling code.

<br>

**Syntax**: `#[error(inline(strategy))]`

<br>

**Available strategies**:

- **`neutral`** (default): Emits `#[inline]`, which hints to the compiler that inlining may be beneficial but leaves the final decision to the optimizer. This is appropriate for most error types.

<br>

- **`always`**: Emits `#[inline(always)]`, which strongly suggests to the compiler that the function should always be inlined. Use this for performance-critical error paths where you want to eliminate function call overhead:

<br>

  ```rust,no_run
  #[derive(Error, Debug)]
  #[error(inline(always))]
  #[error("fast path error: {code}")]
  struct FastError {
      code: u32,
  }
  ```

<br>

- **`never`**: Emits `#[inline(never)]`, which prevents the compiler from inlining the function. Use this to reduce code size when errors are rarely constructed or when you want consistent stack traces:

<br>

  ```rust,no_run
  #[derive(Error, Debug)]
  #[error(inline(never))]
  #[error("rare error condition")]
  struct RareError {
      details: String,
  }
  ```

<br>

The inline strategy applies to all generated methods: `fmt` for Display, `source` for Error, and `from` if `#[error(from)]` is used. When applied at the enum level, it affects all variants.

<br>

### Import Root: `#[error(import(path))]`

By default, all generated code uses `::core` for maximum compatibility with `no_std` environments. The `#[error(import(path))]` attribute overrides this default, allowing you to specify an alternative import root.

<br>

**Syntax**: `#[error(import(::path))]`

<br>

**Common use cases**:

**Using `std` instead of `core`**: When your crate requires the standard library and you want to explicitly use `std::error::Error` and `std::fmt`:

<br>

```rust,no_run
#[derive(Error, Debug)]
#[error(import(::std))]
#[error("standard error")]
struct StdError {
    message: String,
}
```

<br>

This generates code using `::std` instead of the default `::core`:

<br>
```rust,no_run
impl ::std::error::Error for StdError { /* ... */ }
impl ::std::fmt::Display for StdError { /* ... */ }
```

<br>

**Explicit `core` usage**: While `::core` is the default, you can make it explicit:

<br>

```rust,no_run
#[derive(Error, Debug)]
#[error(import(::core))]
#[error("explicit core error")]
struct CoreError {
    msg: String,
}
```

<br>

**Custom preludes**: When integrating with custom preludes or re-exports:

<br>

```rust,no_run
#[derive(Error, Debug)]
#[error(import(crate::prelude))]
#[error("custom prelude error")]
struct CustomError {
    msg: String,
}
```

<br>

The import root affects all generated trait implementations and macro invocations. This attribute is particularly useful when:
- Explicitly targeting `std` in std-only crates
- Making `core` usage explicit for documentation purposes
- Working with custom error trait definitions
- Integrating with frameworks that provide their own error handling primitives

---

## Struct Patterns

### Standalone Errors

Simple errors with custom messages and optional source fields:

<br>

```rust,no_run
#[derive(Error, Debug)]
#[error("operation failed: {reason}")]
struct OperationError {
    reason: String,
}
```

<br>

### Errors with Source

Chaining errors while providing custom context:

<br>

```rust,no_run
#[derive(Error, Debug)]
#[error("database query failed: {}", query)]
#[error(source(cause))]
struct QueryError {
    query: String,
    cause: sqlx::Error,
}
```

<br>

### Transparent Wrappers

New-type wrappers that preserve the inner error's display and source:

<br>

```rust,no_run
#[derive(Error, Debug)]
#[error(transparent(0))]
struct DatabaseError(sqlx::Error);
```

<br>

### Convertible Wrappers

New-types with automatic conversion and custom display:

<br>

```rust,no_run
#[derive(Error, Debug)]
#[error("integer parsing failed")]
#[error(from)]
struct IntError(std::num::ParseIntError);

// Enables: let err: IntError = "abc".parse::<i32>().unwrap_err().into();
```

---

## Enum Patterns

Enums support per-variant attribute configuration, enabling complex error hierarchies with minimal code.

<br>

### Basic Enum

<br>

```rust,no_run
#[derive(Error, Debug)]
enum FileError {
    #[error("file not found: {}", path)]
        NotFound { path: String },

        #[error("permission denied: {}", path)]
        PermissionDenied { path: String },

    #[error("file is a directory")]
    IsDirectory,
}
```

<br>

### Enum with Sources

Each variant can specify its own source field:

<br>

```rust,no_run
#[derive(Error, Debug)]
enum ApplicationError {
    #[error("database error")]
    #[error(source(0))]
    Database(sqlx::Error),

    #[error("network error")]
    #[error(source(io))]
    Network { io: std::io::Error },

    #[error("configuration error: {0}")]
    Config(String),
}
```

<br>

### Enum with Transparent Variants

Mix transparent and custom variants:

<br>

```rust,no_run
#[derive(Error, Debug)]
enum ServerError {
    #[error(transparent(0))]
    Io(std::io::Error),

    #[error(transparent(inner))]
    Database { inner: sqlx::Error },

    #[error("invalid request: {}", message)]
    BadRequest { message: String },
}
```

<br>

### Enum with From Conversions

Enable automatic conversion for specific variants:

<br>

```rust,no_run
#[derive(Error, Debug)]
enum ParseError {
    #[error("integer parse failed")]
    #[error(from)]
    Int(std::num::ParseIntError),

    #[error("float parse failed")]
    #[error(from)]
    Float(std::num::ParseFloatError),

    #[error("syntax error: {0}")]
    Syntax(String),
}

// Enables automatic conversion:
// let err: ParseError = "abc".parse::<i32>().unwrap_err().into();
```

<br>

### Enum-Level Attributes

Type-level attributes apply to all variants:

<br>

```rust,no_run
#[derive(Error, Debug)]
#[error(inline(always))]
#[error(import(::std))]
enum OptimizedError {
    #[error("first variant")]
    First,

    #[error("second variant")]
    Second,
}
```

## Field Reference Syntax

Fields can be referenced by name or position:

<br>

**Named fields**: Use the field's identifier

<br>
```rust,no_run
#[error(source(field_name))]
#[error(transparent(field_name))]
```

<br>

**Tuple fields**: Use zero-based numeric indices

<br>
```rust,no_run
#[error(source(0))]
#[error(transparent(1))]
```

---

## Composability with fack-codegen

The `fack-codegen` crate provides the code generation engine as a standalone library, distinct from the proc-macro interface. This enables integration into custom tooling:

<br>

```rust,no_run
use fack_codegen::{Target, expand::Expand};

// Parse error definition
let target = Target::input(&derive_input)?;

// Generate implementation
let expanded = Expand::target(target)?;
```

<br>

**Use cases**:
- Custom derive macros that extend error functionality
- Build-time code generation without proc-macros
- IDE tooling and code analysis
- Error schema validation and documentation generation

<br>

The separation between `fack-macro` (proc-macro interface) and `fack-codegen` (pure logic) ensures the generation logic remains accessible in contexts where proc-macros cannot be used.

---

## Type Support

**Supported**: Structs (named, tuple, unit) and enums with any variant style

<br>

**Unsupported**: Union types (unions cannot implement `Error`)

<br>

## Generated Code Guarantees

All implementations include:
- `#[automatically_derived]` attribute for tool recognition and linter warning supression
- Proper destructuring of fields in pattern matches
- Correct forwarding of lifetimes and generic parameters
- No unsafe code
- No heap allocations in generated methods (all `no_std`-compatible!)

## License

Copyright (C) 2025 W. Frakchi

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

See [the full license agreement](LICENSE.md) for further information.
