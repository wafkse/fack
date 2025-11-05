# fack

Declarative error handling for Rust with `no_std` support and zero-allocation runtime.

```rust,no_run
use fack::prelude::*;

#[derive(Error, Debug)]
#[error("configuration error: {message}")]
struct ConfigError {
    message: String,
}
```

## Features

- **no_std compatible**: Uses `::core` by default, optional `::std` support
- **Zero allocation**: No heap allocations in generated error implementations
- **Declarative attributes**: Configure errors through `#[error(...)]` attributes
- **Composable codegen**: Standalone `fack-codegen` library for custom tooling
- **Span preservation**: Accurate IDE hints and LSP integration

## Installation

```toml
[dependencies]
fack = "0.1.0"
```

## Quick Start

```rust,no_run
use fack::prelude::*;

// Simple error with display message
#[derive(Error, Debug)]
#[error("file not found: {path}")]
struct FileError {
    path: String,
}

// Error with source
#[derive(Error, Debug)]
#[error("network request failed")]
#[error(source(inner))]
struct NetworkError {
    inner: std::io::Error,
    url: String,
}

// Enum with multiple variants
#[derive(Error, Debug)]
enum AppError {
    #[error("io error")]
    #[error(from)]
    Io(std::io::Error),
    
    #[error("parse failed: {msg}")]
    Parse { msg: String },
}
```

## Attribute Reference

- **`#[error("format {field}")]`**: Display implementation - named fields use `{field}`, tuple fields use `{_0}`
- **`#[error(source(field))]`**: Designate error source for chaining
- **`#[error(transparent(field))]`**: Forward Display and source to inner field
- **`#[error(from)]`**: Generate `From` implementation for automatic conversion
- **`#[error(inline(strategy))]`**: Control inlining (`neutral`, `always`, `never`)
- **`#[error(import(path))]`**: Override default `::core` import root

See [documentation](https://docs.rs/fack) for detailed usage and examples.

## Architecture

- **fack**: Main crate with derive macro
- **fack-core**: Core trait definitions
- **fack-macro**: Procedural macro implementation
- **fack-codegen**: Standalone code generation engine

## License

Copyright (C) 2025 W. Frakchi

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

See [the full license agreement](LICENSE.md) for further information.