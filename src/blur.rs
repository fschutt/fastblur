use std::cmp::min;
use std::iter::repeat;

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

/// How the three stretches of a blur pass split up.
///
/// Averaging over a window that is clamped at both ends means
///
///     out[0] = ((radius + 1) * first + src[1 ..= radius].sum()) / (2 * radius + 1)
///     out[i] = out[i - 1] + src[i + radius] - src[i - radius - 1]
///
/// with both of those indices clamped to the ends of the line. Only the first
/// and last stretches actually need the clamping: over the middle - which is
/// all of a real image - both ends of the window are inside the line, so it
/// becomes a plain `zip` of three slices with nothing to check and nothing to
/// branch on.
#[inline]
fn split(len: usize, radius: usize) -> (usize, usize) {
    (min(len, radius + 1), len.saturating_sub(2 * radius + 1))
}

/// Window sum for the first output pixel, primed one step short: the first turn
/// of the caller's loop brings `src[radius]` in and drops the clamped
/// `src[-radius - 1]`, which completes it.
#[inline]
fn prime<const CH: usize>(src: &[[u8; CH]], first: [u8; CH], last: [u8; CH], radius: usize) -> [isize; CH] {
    let mut val = [0isize; CH];
    for c in 0..CH {
        val[c] = (radius as isize + 1) * isize::from(first[c]);
    }
    for px in src.iter().take(radius) {
        for c in 0..CH { val[c] += isize::from(px[c]); }
    }
    // A window wider than the line reaches past the end, onto the last pixel.
    if radius > src.len() {
        for c in 0..CH {
            val[c] += (radius - src.len()) as isize * isize::from(last[c]);
        }
    }
    val
}

/// Runs one horizontal and one vertical box pass over `buf`, using `scratch`
/// as the intermediate. The finished image ends up in `buf`.
#[inline]
fn box_blur<const CH: usize>(scratch: &mut [[u8; CH]], buf: &mut [[u8; CH]], width: usize, height: usize, blur_radius_horz: usize, blur_radius_vert: usize)
{
    box_blur_horz(buf, scratch, width, height, blur_radius_horz);
    box_blur_vert(scratch, buf, width, height, blur_radius_vert);
}

#[inline]
fn box_blur_horz<const CH: usize>(src: &[[u8; CH]], dst: &mut [[u8; CH]], width: usize, height: usize, blur_radius: usize)
{
    let src = &src[..width * height];
    let dst = &mut dst[..width * height];

    if blur_radius == 0 {
        dst.copy_from_slice(src);
        return;
    }

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;

    // Rows are contiguous, so `chunks_exact` hands out exactly one each.
    for (src_row, dst_row) in src.chunks_exact(width).zip(dst.chunks_exact_mut(width)) {
        blur_row(src_row, dst_row, blur_radius, iarr);
    }
}

#[inline]
fn blur_row<const CH: usize>(src: &[[u8; CH]], dst: &mut [[u8; CH]], radius: usize, iarr: f32)
{
    let len = src.len();
    let first = src[0];
    let last = src[len - 1];
    let (n1, n2) = split(len, radius);

    let mut val = prime(src, first, last, radius);

    // Left edge: the pixel leaving the window is the clamped first one.
    let entering = src.iter().copied().skip(radius).chain(repeat(last));
    for (inc, out) in entering.zip(dst[..n1].iter_mut()) {
        for c in 0..CH {
            val[c] += isize::from(inc[c]) - isize::from(first[c]);
        }
        *out = average(&val, iarr);
    }

    // Middle: both ends of the window are inside the row.
    for ((inc, dec), out) in src[min(2 * radius + 1, len)..].iter()
        .zip(src.iter())
        .zip(dst[n1..].iter_mut())
    {
        for c in 0..CH {
            val[c] += isize::from(inc[c]) - isize::from(dec[c]);
        }
        *out = average(&val, iarr);
    }

    // Right edge: the pixel entering the window is the clamped last one.
    for (dec, out) in src[n2..].iter().zip(dst[n1 + n2..].iter_mut()) {
        for c in 0..CH {
            val[c] += isize::from(last[c]) - isize::from(dec[c]);
        }
        *out = average(&val, iarr);
    }
}

/// Blurs vertically by sweeping down the image, keeping one running window sum
/// per column.
///
/// Transcribing the horizontal pass directly would walk each column with a
/// stride of `width`, which touches one pixel per cache line. Going row by row
/// instead keeps every read and write contiguous and lets the per-column sums
/// be updated by zipping whole rows together.
#[inline]
fn box_blur_vert<const CH: usize>(src: &[[u8; CH]], dst: &mut [[u8; CH]], width: usize, height: usize, blur_radius: usize)
{
    let src = &src[..width * height];
    let dst = &mut dst[..width * height];

    if blur_radius == 0 {
        dst.copy_from_slice(src);
        return;
    }

    let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;
    let (n1, n2) = split(height, blur_radius);
    let first = &src[..width];
    let last = &src[width * (height - 1)..];

    let mut acc = vec![[0isize; CH]; width];
    for (a, px) in acc.iter_mut().zip(first) {
        for c in 0..CH {
            a[c] = (blur_radius as isize + 1) * isize::from(px[c]);
        }
    }
    for row in src.chunks_exact(width).take(blur_radius) {
        for (a, px) in acc.iter_mut().zip(row) {
            for c in 0..CH { a[c] += isize::from(px[c]); }
        }
    }
    if blur_radius > height {
        let extra = (blur_radius - height) as isize;
        for (a, px) in acc.iter_mut().zip(last) {
            for c in 0..CH { a[c] += extra * isize::from(px[c]); }
        }
    }

    // Top edge: the row leaving the window is the clamped first one.
    let entering = src.chunks_exact(width).skip(blur_radius).chain(repeat(last));
    for (inc, out_row) in entering.zip(dst.chunks_exact_mut(width).take(n1)) {
        for (((a, i), f), out) in acc.iter_mut().zip(inc).zip(first).zip(out_row.iter_mut()) {
            for c in 0..CH { a[c] += isize::from(i[c]) - isize::from(f[c]); }
            *out = average(a, iarr);
        }
    }

    // Middle: the window sits entirely inside the image.
    for ((inc, dec), out_row) in src.chunks_exact(width).skip(min(2 * blur_radius + 1, height))
        .zip(src.chunks_exact(width))
        .zip(dst.chunks_exact_mut(width).skip(n1))
    {
        for (((a, i), d), out) in acc.iter_mut().zip(inc).zip(dec).zip(out_row.iter_mut()) {
            for c in 0..CH { a[c] += isize::from(i[c]) - isize::from(d[c]); }
            *out = average(a, iarr);
        }
    }

    // Bottom edge: the row entering the window is the clamped last one.
    for (dec, out_row) in src.chunks_exact(width).skip(n2)
        .zip(dst.chunks_exact_mut(width).skip(n1 + n2))
    {
        for (((a, l), d), out) in acc.iter_mut().zip(last).zip(dec).zip(out_row.iter_mut()) {
            for c in 0..CH { a[c] += isize::from(l[c]) - isize::from(d[c]); }
            *out = average(a, iarr);
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
