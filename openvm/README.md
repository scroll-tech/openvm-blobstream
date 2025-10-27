# openvm guest example

This directory contains an example of a minimal blobstream verifier.

## Building the example

in the `program` directory, run:

```sh
OPENVM_RUST_TOOLCHAIN=nightly-2025-08-18 cargo openvm build
```

in the `script` directory, run:

```sh
cargo run --release
```
