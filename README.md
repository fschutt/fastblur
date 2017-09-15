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
