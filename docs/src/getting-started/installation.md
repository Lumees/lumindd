# Installation

## Adding lumindd to Your Project

Add lumindd as a dependency in your `Cargo.toml`:

```toml
[dependencies]
lumindd = "1.0"
```

Then run `cargo build` to download and compile the crate.

## Minimum Supported Rust Version

lumindd requires **Rust 1.70** or later. You can check your installed version with:

```sh
rustc --version
```

If you need to update:

```sh
rustup update stable
```

## Feature Flags

lumindd currently has no optional feature flags. The entire API is available by default. The only runtime dependency is the [`bitflags`](https://crates.io/crates/bitflags) crate (version 2), which is pulled in automatically.

## Building from Source

To build the library from a local checkout of the repository:

```sh
git clone https://github.com/Lumees/lumindd.git
cd lumindd
cargo build --release
```

To run the test suite:

```sh
cargo test
```

To generate the rustdoc API documentation locally:

```sh
cargo doc --open
```

## Verifying the Installation

Create a small test program to confirm everything is working:

```rust
use lumindd::Manager;

fn main() {
    let mut mgr = Manager::new();
    let x = mgr.bdd_new_var();
    let y = mgr.bdd_new_var();

    let f = mgr.bdd_and(x, y);
    assert_eq!(mgr.bdd_count_minterm(f, 2), 1.0);

    println!("lumindd is working. BDD nodes: {}", mgr.num_nodes());
}
```

Save this as `src/main.rs` in a new Cargo project, add the `lumindd` dependency, and run with `cargo run`. You should see output like:

```
lumindd is working. BDD nodes: 4
```

## Platform Support

lumindd is pure Rust with no platform-specific code, C dependencies, or build scripts. It compiles on any target supported by the Rust toolchain, including Linux, macOS, Windows, and WebAssembly.
