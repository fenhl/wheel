//! This crate contains boilerplate that is useful in almost every Rust crate.

#![deny(
    missing_docs,
    rust_2018_idioms, // this lint is actually about idioms that are *outdated* in Rust 2018
    unused,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    warnings
)]

pub use wheel_derive::{
    bin,
    lib,
    main
};

#[doc(hidden)] pub use {
    paw,
    tokio
}; // used in proc macro
