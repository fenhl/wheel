#!/bin/sh

set -e

cargo check
cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features
cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features --features=github
cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features --features=racetime
cargo check --manifest-path=crate/wheel/Cargo.toml --all-features
cargo doc
