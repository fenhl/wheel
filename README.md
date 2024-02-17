This is a [Rust](https://rust-lang.org/) library crate that provides boilerplate common to most of my Rust projects.

The most important feature is an attribute `#[wheel::main]` which annotates the `main` function to add support for both [clap](https://docs.rs/clap) and [tokio](https://docs.rs/tokio) at the same time.
