# cargo-single-pyo3

Utility to build Python modules from a single Rust files via [pyo3](https://github.com/PyO3/pyo3). Inspired by [cargo-single](https://github.com/inejge/cargo-single).

## Installation

```
cargo install cargo-single-pyo3
```

## Example

First, create a single Rust file with a pyo3 module. Add any dependencies as double-slash comments at the top of the file. For example, if you create `foo.rs` with the contents:

```rust
// rand = "*"

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
  let c = rand::thread_rng().gen_range(0 ..= 1);
  Ok((a + b + c).to_string())
}

#[pymodule]
fn foo(py: Python, m: &PyModule) -> PyResult<()> {
  m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
  Ok(())
}
```

Then run:

```
cargo single-pyo3 foo.rs
``` 

This should generate a file `foo.so`, which you can then import:

```
$ python3
>>> import foo
>>> foo.sum_as_string(1, 2)
'3'
>>> foo.sum_as_string(1, 2)
'4'
```

## Usage notes

**Module name:** the name of the file is the name of the module, e.g. `foo.rs` generates `foo.so`. The name of the `#[pymodule]` function must be the same.

**Build process:** the tool creates a Cargo project in your temporary directory that is associated with the module name, e.g. `/tmp/foo`. This could cause any usual problems of conflicts between users or projects on the same machine, so be careful (or submit a PR if you have a different preference).

**Pyo3 version:** the Cargo dependency on pyo3 is automatically generated. If you need to change the version, use the `--pyo3` flag, e.g. `--pyo3 0.13`. You can also use `--pyo3 github` to use the latest on main branch. At the time of writing, this option was necessary to build on OS X.