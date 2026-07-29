use std::cmp::min;

/// Widest box we will ever hand out.
///
/// `f32 as i32` saturates at `i32::MAX`, so without a clamp the `wu = wl + 2`
/// in `create_box_gauss` overflows for absurd sigmas. 2^20 is exactly
/// representable in `f32`, is far wider than any real image, and keeps the
/// index arithmetic in the blur passes well inside `usize` on 32-bit targets.
const MAX_BOX_WIDTH: i32 = 1 << 20;

/// Turns a box width into the radius the blur passes work with, clamped to the
/// size of the axis it is applied to.
///
/// Once the box is as wide as the image every extra pixel of reach only piles
/// more weight onto the clamped edge pixel, so cutting it off there keeps the
/// result sane while bounding `radius * width` for the index arithmetic.
#[inline]
fn box_radius(box_size: i32, axis_len: usize) -> usize {
    min(((box_size - 1) / 2) as usize, axis_len)
}

/// Blurs an RGB image in place.
pub fn gaussian_blur(data: &mut [[u8; 3]], width: usize, height: usize, blur_radius: f32)
{
    gaussian_blur_asymmetric(data, width, height, blur_radius, blur_radius)
}

/// Same as gaussian_blur, but allows using different blur radii for vertical and horizontal passes
pub fn gaussian_blur_asymmetric(data: &mut [[u8; 3]], width: usize, height: usize, blur_radius_horizontal: f32, blur_radius_vertical: f32)
{
    gaussian_blur_asymmetric_generic(data, width, height, blur_radius_horizontal, blur_radius_vertical)
}

/// Blurs an image with `CH` channels per pixel in place.
///
/// `CH` is a const parameter, so this covers grayscale (`[u8; 1]`), gray+alpha
/// (`[u8; 2]`), RGB (`[u8; 3]`), RGBA and BGRA (`[u8; 4]`) and anything wider,
/// with the per-channel loops unrolled at compile time.
///
/// Every channel is blurred independently, which is what you want for
/// **premultiplied** alpha. Blurring straight (unpremultiplied) RGBA bleeds
/// the colour of fully transparent pixels into their visible neighbours, so
/// premultiply first and unpremultiply afterwards.
pub fn gaussian_blur_generic<const CH: usize>(data: &mut [[u8; CH]], width: usize, height: usize, blur_radius: f32)
{
    gaussian_blur_asymmetric_generic(data, width, height, blur_radius, blur_radius)
}

/// Same as gaussian_blur_generic, but allows using different blur radii for vertical and horizontal passes
pub fn gaussian_blur_asymmetric_generic<const CH: usize>(data: &mut [[u8; CH]], width: usize, height: usize, blur_radius_horizontal: f32, blur_radius_vertical: f32)
{
    if width == 0 || height == 0 { return; }

    let boxes_horz = create_box_gauss::<3>(blur_radius_horizontal);
    let boxes_vert = create_box_gauss::<3>(blur_radius_vertical);
    let mut scratch = data.to_vec();

    for (box_size_horz, box_size_vert) in boxes_horz.iter().zip(boxes_vert.iter()) {
        let radius_horz = box_radius(*box_size_horz, width);
        let radius_vert = box_radius(*box_size_vert, height);
        box_blur(&mut scratch, data, width, height, radius_horz, radius_vert);
    }
}

/// Same as gaussian_blur_asymmetric, but for a single channel image such as a
/// mask or a standalone alpha plane.
pub fn gaussian_blur_asymmetric_single_channel(data: &mut [u8], width: usize, height: usize, blur_radius_horizontal: f32, blur_radius_vertical: f32)
{
    // SAFETY: `[u8; 1]` is guaranteed to have the same size and alignment as
    // `u8` - an array is N consecutive elements with the alignment of the
    // element type - so this is a view of the same bytes with the same length,
    // not a reinterpretation of them.
    let pixels: &mut [[u8; 1]] = unsafe {
        ::std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut [u8; 1], data.len())
    };

    gaussian_blur_asymmetric_generic(pixels, width, height, blur_radius_horizontal, blur_radius_vertical)
}

#[inline]
/// Returns the widths of the `N` box blur passes that together approximate a
/// gaussian blur of the given sigma.
///
/// If there is no valid size (the sigma is zero, negative or NaN) this returns
/// `[1; N]`, which means a blur radius of 0.
pub(crate) fn create_box_gauss<const N: usize>(sigma: f32) -> [i32; N]
{
    let mut sizes = [1i32; N];

    // NaN compares false here and so keeps the "don't blur" widths; an infinite
    // sigma does not, and is caught by the `MAX_BOX_WIDTH` clamp below.
    if sigma > 0.0 {
        let n_float = N as f32;

        // Ideal averaging filter width.
        //
        // The `+ 1` belongs *inside* the square root - see `boxesForGauss` in
        // http://blog.ivank.net/fastest-gaussian-blur.html. Adding it outside
        // pushes `w_ideal` up by just under one, which is enough to make
        // `floor()` land on the next box width for practically every sigma, so
        // all three passes ended up using the same (too wide) box instead of
        // the intended two narrow ones plus one wide one.
        let w_ideal = (12.0 * sigma * sigma / n_float + 1.0).sqrt();
        // Clamp *before* the cast: `f32 as i32` saturates at `i32::MAX`, which
        // used to make `wl + 2` below overflow and panic in debug builds.
        let mut wl: i32 = w_ideal.floor().min(MAX_BOX_WIDTH as f32) as i32;

        if wl % 2 == 0 { wl -= 1; };

        let wu = wl + 2;

        let wl_float = wl as f32;
        let m_ideal = (12.0 * sigma * sigma - n_float * wl_float * wl_float - 4.0 * n_float * wl_float - 3.0 * n_float) /
                    (-4.0 * wl_float - 4.0);
        let m: usize = m_ideal.round() as usize;

        for (i, size) in sizes.iter_mut().enumerate() {
            *size = if i < m { wl } else { wu };
        }
    }

    sizes
}

/// Runs one horizontal and one vertical box pass over `buf`, using `scratch`
/// as the intermediate. The finished image ends up in `buf`.
#[inline]
fn box_blur<const CH: usize>(scratch: &mut [[u8; CH]], buf: &mut [[u8; CH]], width: usize, height: usize, blur_radius_horz: usize, blur_radius_vert: usize)
{
    box_blur_horz(buf, scratch, width, height, blur_radius_horz);
    box_blur_vert(scratch, buf, width, height, blur_radius_vert);
}

/// Divides the running window sum by the window size and packs it back into a pixel.
#[inline]
fn average<const CH: usize>(val: &[isize; CH], iarr: f32) -> [u8; CH]
{
    let mut out = [0u8; CH];
    for c in 0..CH {
        out[c] = round(val[c] as f32 * iarr) as u8;
    }
    out
}

#[inline]
fn box_blur_vert<const CH: usize>(src: &[[u8; CH]], dst: &mut [[u8; CH]], width: usize, height: usize, blur_radius: usize)
{
    if blur_radius == 0 {
        dst.copy_from_slice(src);
        return;
    }

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;

    for i in 0..width {

        let col_start = i; //inclusive
        let col_end = i + width * (height - 1); //inclusive
        let mut ti: usize = i;
        let mut li: usize = ti;
        let mut ri: usize = ti + blur_radius * width;

        let fv: [u8; CH] = src[col_start];
        let lv: [u8; CH] = src[col_end];

        let mut val = [0isize; CH];
        for c in 0..CH {
            val[c] = (blur_radius as isize + 1) * isize::from(fv[c]);
        }

        // Get the pixel at the specified index, or the first pixel of the column
        // if the index is beyond the top edge of the image
        let get_top = |i: usize| {
            if i < col_start {
                fv
            } else {
                src[i]
            }
        };

        // Get the pixel at the specified index, or the last pixel of the column
        // if the index is beyond the bottom edge of the image
        let get_bottom = |i: usize| {
            if i > col_end {
                lv
            } else {
                src[i]
            }
        };

        for j in 0..min(blur_radius, height) {
            let bb = src[ti + j * width];
            for c in 0..CH { val[c] += isize::from(bb[c]); }
        }
        if blur_radius > height {
            for c in 0..CH {
                val[c] += (blur_radius - height) as isize * isize::from(lv[c]);
            }
        }

        for _ in 0..min(height, blur_radius + 1) {
            let bb = get_bottom(ri); ri += width;
            for c in 0..CH { val[c] += isize::from(bb[c]) - isize::from(fv[c]); }

            dst[ti] = average(&val, iarr);
            ti += width;
        }

        if height > blur_radius { // otherwise `(height - blur_radius)` will underflow
            for _ in (blur_radius + 1)..(height - blur_radius) {

                let bb1 = src[ri]; ri += width;
                let bb2 = src[li]; li += width;

                for c in 0..CH { val[c] += isize::from(bb1[c]) - isize::from(bb2[c]); }

                dst[ti] = average(&val, iarr);
                ti += width;
            }

            for _ in 0..min(height - blur_radius - 1, blur_radius) {
                let bb = get_top(li); li += width;

                for c in 0..CH { val[c] += isize::from(lv[c]) - isize::from(bb[c]); }

                dst[ti] = average(&val, iarr);
                ti += width;
            }
        }
    }
}

#[inline]
fn box_blur_horz<const CH: usize>(src: &[[u8; CH]], dst: &mut [[u8; CH]], width: usize, height: usize, blur_radius: usize)
{
    if blur_radius == 0 {
        dst.copy_from_slice(src);
        return;
    }

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;

    for i in 0..height {

        let row_start: usize = i * width; // inclusive
        let row_end: usize = (i + 1) * width - 1; // inclusive
        let mut ti: usize = i * width; // VERTICAL: $i;
        let mut li: usize = ti;
        let mut ri: usize = ti + blur_radius;

        let fv: [u8; CH] = src[row_start];
        let lv: [u8; CH] = src[row_end]; // VERTICAL: $src[ti + $width - 1];

        let mut val = [0isize; CH];
        for c in 0..CH {
            val[c] = (blur_radius as isize + 1) * isize::from(fv[c]);
        }

        // Get the pixel at the specified index, or the first pixel of the row
        // if the index is beyond the left edge of the image
        let get_left = |i: usize| {
            if i < row_start {
                fv
            } else {
                src[i]
            }
        };

        // Get the pixel at the specified index, or the last pixel of the row
        // if the index is beyond the right edge of the image
        let get_right = |i: usize| {
            if i > row_end {
                lv
            } else {
                src[i]
            }
        };

        for j in 0..min(blur_radius, width) {
            let bb = src[ti + j]; // VERTICAL: ti + j * width
            for c in 0..CH { val[c] += isize::from(bb[c]); }
        }
        // The row is `width` long, so this has to subtract `width`; subtracting
        // `height` (a copy-paste slip from the vertical pass) underflows as
        // soon as the image is taller than the blur box is wide.
        if blur_radius > width {
            for c in 0..CH {
                val[c] += (blur_radius - width) as isize * isize::from(lv[c]);
            }
        }


        // Process the left side where we need pixels from beyond the left edge
        for _ in 0..min(width, blur_radius + 1) {
            let bb = get_right(ri); ri += 1;
            for c in 0..CH { val[c] += isize::from(bb[c]) - isize::from(fv[c]); }

            dst[ti] = average(&val, iarr);
            ti += 1; // VERTICAL : ti += width, same with the other areas
        }

        if width > blur_radius { // otherwise `(width - blur_radius)` will underflow
            // Process the middle where we know we won't bump into borders
            // without the extra indirection of get_left/get_right. This is faster.
            for _ in (blur_radius + 1)..(width - blur_radius) {

                let bb1 = src[ri]; ri += 1;
                let bb2 = src[li]; li += 1;

                for c in 0..CH { val[c] += isize::from(bb1[c]) - isize::from(bb2[c]); }

                dst[ti] = average(&val, iarr);
                ti += 1;
            }

            // Process the right side where we need pixels from beyond the right edge
            for _ in 0..min(width - blur_radius - 1, blur_radius) {
                let bb = get_left(li); li += 1;

                for c in 0..CH { val[c] += isize::from(lv[c]) - isize::from(bb[c]); }

                dst[ti] = average(&val, iarr);
                ti += 1;
            }
        }
    }
}

#[inline]
/// Fast rounding for x <= 2^23.
/// This is orders of magnitude faster than built-in rounding intrinsic.
///
/// Source: https://stackoverflow.com/a/42386149/585725
fn round(mut x: f32) -> f32 {
    x += 12582912.0;
    x -= 12582912.0;
    x
}
