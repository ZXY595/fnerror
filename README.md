# fnerror
A Rust library for error handling when you want your functions to automatically generate errors.
# Features
- Define error types directly in function expressions.
- Generate error types binding to the function.

# Usage 
Add the following to your Cargo.toml:
```sh
cargo add fnerror
```
You also need to add thiserror to your crate: (Will be unneeded in the future)
```sh
cargo add thiserror
```

# Example
```rust
#[fnerror]
fn foo() -> Result<(), MyError> /* or Result<T> to use the default ident of error type ,
which is pascal case of the function name + "Error", like `FooError` */
{
    bar().map_err(|e| {
        #[fnerr]
        Error2("{}", e as String)
    })?;
    baz().map_err(|e| {
        #[fnerr]
        Error3("{}, {}", e as &'static str, 123 as u8)
    })?;
    Ok(())
}

fn bar() -> Result<(), String> {
    Err("test2 error".to_string())
}

fn baz() -> Result<(), &'static str> {
    Err("test2 error")
}
```

Which expands to (with thiserror feature):

```rust
fn foo() -> ::std::result::Result<(), MyError> {
    bar().map_err(|e| MyError::Error2(e))?;
    baz().map_err(|e| MyError::Error3(e, 123))?;
    Ok(())
}
#[derive(Debug, ::thiserror::Error)]
pub enum MyError {
    #[error("{}", 0usize)]
    Error2(String),
    #[error("{}, {}",0usize, 1usize)]
    Error3(&'static str, u8),
}
```

# Status
- [x] Parse AST to generate error types.
- [x] Generate error implementations using thiserror crate.
- [x] Support generic error types (experimental).
- [x] Support custom error name.
- [x] Replace panics with errors.
- [ ] Support other error implementations.
- [ ] Support more formatting options.
