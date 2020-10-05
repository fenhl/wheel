This is a [Rust](https://rust-lang.org/) library crate that provides boilerplate common to most of my Rust projects.

Currently, the only tested feature is an attribute `#[wheel::main]` which annotates the `main` function to add support for both [paw](https://docs.rs/paw) and [tokio](https://docs.rs/tokio), which is currently not possible by simply stacking `#[paw::main]` and `#[tokio::main]`.
