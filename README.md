# fastblur

Fast (linear time) implementation of the Gaussian Blur algorithm in Rust.
Original taken from http://blog.ivank.net/fastest-gaussian-blur.html

## Usage

The image is assumed to be an RGB image with three channels.
This should change in the future, so you can blur as many channels as you want. Still very WIP.

```rust
#[dependencies]
fastblur = { git = "https://github.com/fschutt/fastblur" }
```

```rust
use fastblur::gaussian_blur;

// data is a Vec<[u8;3]> - 3 items for R, G and B.
// This format will probably change.
gaussian_blur(&mut data, width, height, 10.0);
```

__NOTE__: This currently crashes with large amounts of pixels. It is also not
"the fastest" Gaussian blur. It currently takes 22ms - but it is independent of the
blur size. A regular Gaussian blur depends on the size of the blur. At a 3px blur,
the example from the `imageproc` library needs 4ms. At a 10px blur, it already needs
28px. And so on. This library always needs 22ms, no matter of the size of the blur.
