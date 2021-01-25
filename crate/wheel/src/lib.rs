//! This crate contains boilerplate that is useful in almost every Rust crate.

#![deny(
    missing_docs,
    rust_2018_idioms, // this lint is actually about idioms that are *outdated* in Rust 2018
    unused,
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    warnings,
)]

pub use wheel_derive::{
    bin,
    lib,
    main,
};

// used in proc macro:
#[doc(hidden)] pub use paw;
#[cfg(feature = "tokio")] #[doc(hidden)] pub use tokio;
#[cfg(feature = "tokio02")] #[doc(hidden)] pub use tokio02 as tokio;
#[cfg(feature = "tokio03")] #[doc(hidden)] pub use tokio03 as tokio;
