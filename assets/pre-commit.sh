#!/bin/sh

set -e

cargo check
cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features
cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features --features=github
cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features --features=racetime
cargo check --manifest-path=crate/wheel/Cargo.toml --all-features
cargo doc
cargo doc --all-features
nix build --no-link
nix eval --expr "$(git show :crate/wheel-cli/assets/flake-bin.nix)" > /dev/null
nix eval --expr "$(git show :crate/wheel-cli/assets/flake-lib.nix)" > /dev/null
