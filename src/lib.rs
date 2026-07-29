#![cfg_attr(all(test, feature = "nightly"), feature(test))]

//! # fastblur
//!
//! Fast (linear time) implementation of the Gaussian Blur algorithm in Rust.
//! Original taken from http://blog.ivank.net/fastest-gaussian-blur.html
//!
//! ## Usage
//!
//! ```rust,ignore
//! [dependencies]
//! fastblur = "0.2"
//! ```
//!
//! ```rust,ignore
//! use fastblur::gaussian_blur;
//!
//! // data is a Vec<[u8; 3]> - 3 items for R, G and B.
//! gaussian_blur(&mut data, width, height, 10.0);
//! ```
//!
//! Any other channel count goes through `gaussian_blur_generic`, which is
//! generic over the number of channels per pixel:
//!
//! ```rust,ignore
//! use fastblur::gaussian_blur_generic;
//!
//! // data is a Vec<[u8; 4]> of *premultiplied* RGBA
//! gaussian_blur_generic(&mut data, width, height, 10.0);
//! ```

mod blur;

#[cfg(test)]
mod test;
// `#[bench]` is still unstable, so the benchmarks only exist on nightly:
// `cargo +nightly bench --features nightly`
#[cfg(all(test, feature = "nightly"))]
mod bench;

pub mod utils;

pub use blur::{
    gaussian_blur,
    gaussian_blur_asymmetric,
    gaussian_blur_generic,
    gaussian_blur_asymmetric_generic,
    gaussian_blur_asymmetric_single_channel,
};

