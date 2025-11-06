#!/bin/bash

set -ex

PWD=$(pwd)

# comment out the following line if you have already has openvm installed
cargo +nightly install cargo-openvm --git https://github.com/openvm-org/openvm.git --locked --tag v1.4.1
cargo openvm setup

pushd "$PWD/openvm/program"
OPENVM_RUST_TOOLCHAIN=nightly-2025-08-18 cargo openvm build
popd

pushd "$PWD/openvm/script"
RUST_LOG=info cargo run --release
cargo openvm verify stark --app-commit app-commit.json
